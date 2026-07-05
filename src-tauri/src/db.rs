use std::path::{Path, PathBuf};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

/// Resolve the on-disk path for the SQLite database file.
///
/// Prefers the user's Documents directory (so the DB is easy to find and
/// back up via cloud sync). Falls back to the platform app-data directory
/// if the Documents directory cannot be resolved. The DB lives under a
/// `BudgetPlanner/` subfolder, whose parent directory is created if missing.
pub fn resolve_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .document_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| format!("failed to resolve a base directory for the database: {e}"))?;

    let dir = base_dir.join("BudgetPlanner");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create database directory {dir:?}: {e}"))?;

    Ok(dir.join("budget.db"))
}

/// The current schema version produced by [`init`].
const CURRENT_SCHEMA_VERSION: i64 = 4;

/// Initialise a connection: configure pragmas and run pending migrations.
///
/// CRITICAL: `journal_mode = DELETE` disables WAL so no `-wal`/`-shm`
/// sidecar files are created — those fight cloud sync (Dropbox/iCloud)
/// which can corrupt a WAL-mode database.
pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    // `journal_mode` returns a row, so use a query rather than execute.
    conn.query_row("PRAGMA journal_mode = DELETE;", [], |_| Ok(()))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    run_migrations(conn)?;
    Ok(())
}

/// Apply schema migrations, gated on `PRAGMA user_version`.
///
/// New versions can be appended as additional `if version < N` blocks.
fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;

    // Each step runs independently and bumps the version, so a database at any
    // older version catches up through every intermediate migration.
    // `user_version` does not accept a bound parameter; the literal is an
    // internal constant, not user input.
    if version < 1 {
        migrate_v1(conn)?;
        conn.execute_batch("PRAGMA user_version = 1;")?;
    }
    if version < 2 {
        migrate_v2(conn)?;
        conn.execute_batch("PRAGMA user_version = 2;")?;
    }
    if version < 3 {
        migrate_v3(conn)?;
        conn.execute_batch("PRAGMA user_version = 3;")?;
    }
    if version < 4 {
        migrate_v4(conn)?;
        conn.execute_batch("PRAGMA user_version = 4;")?;
    }

    debug_assert_eq!(
        conn.query_row("PRAGMA user_version;", [], |row| row.get::<_, i64>(0))?,
        CURRENT_SCHEMA_VERSION
    );
    Ok(())
}

/// v1 schema: core tables. Money is always stored as INTEGER minor units
/// (cents); never REAL, to avoid floating-point rounding errors.
fn migrate_v1(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS app_profiles (
            id           INTEGER PRIMARY KEY,
            profile_name TEXT NOT NULL,
            created_at   TEXT NOT NULL,
            db_sync_path TEXT
        );

        CREATE TABLE IF NOT EXISTS accounts (
            id         INTEGER PRIMARY KEY,
            profile_id INTEGER NOT NULL REFERENCES app_profiles(id),
            name       TEXT NOT NULL,
            balance    INTEGER NOT NULL DEFAULT 0,
            currency   TEXT NOT NULL DEFAULT 'USD',
            type       TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS categories (
            id         INTEGER PRIMARY KEY,
            profile_id INTEGER NOT NULL REFERENCES app_profiles(id),
            name       TEXT NOT NULL,
            color_code TEXT,
            type       TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS budgets (
            id          INTEGER PRIMARY KEY,
            profile_id  INTEGER NOT NULL REFERENCES app_profiles(id),
            category_id INTEGER NOT NULL REFERENCES categories(id),
            period      TEXT NOT NULL DEFAULT 'monthly',
            amount      INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS transactions (
            id                  INTEGER PRIMARY KEY,
            account_id          INTEGER NOT NULL REFERENCES accounts(id),
            amount              INTEGER NOT NULL,
            category_id         INTEGER REFERENCES categories(id),
            date                TEXT NOT NULL,
            note                TEXT,
            type                TEXT NOT NULL,
            transfer_account_id INTEGER REFERENCES accounts(id)
        );
        ",
    )
}

/// v2: per-budget alert threshold (fraction of the limit at which a budget is
/// considered "near" — e.g. 0.9 = 90%). Added via ALTER so existing budgets
/// keep working.
fn migrate_v2(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "ALTER TABLE budgets ADD COLUMN alert_threshold REAL NOT NULL DEFAULT 0.9;",
    )
}

/// v3: one level of category nesting. `parent_id` references another category
/// (NULL = top-level). Spending in a sub-category rolls up to the parent's
/// budget on the frontend.
fn migrate_v3(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "ALTER TABLE categories ADD COLUMN parent_id INTEGER REFERENCES categories(id);",
    )
}

/// v4: an explicit `opening_balance` column on `accounts`, laying the
/// groundwork for a balance-recompute maintenance command (see
/// `commands::recompute_balances`). Previously the starting balance was only
/// ever baked into `balance` at account creation and had no independent
/// record, so `balance` alone could not be safely rebuilt from transaction
/// history without wiping it.
///
/// After adding the column (defaulting new/existing rows to 0), backfill each
/// existing account's `opening_balance` by subtracting that account's net
/// transaction effect from its current `balance` — recovering the presumed
/// starting balance at migration time. The net effect mirrors
/// `commands::adjust_balances` exactly: income adds, expense subtracts,
/// transfer subtracts from the source and adds to the destination.
///
/// `opening_balance` is intentionally NOT exposed on the `Account` struct or
/// IPC surface — it is an internal detail of the recompute SQL.
fn migrate_v4(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("ALTER TABLE accounts ADD COLUMN opening_balance INTEGER NOT NULL DEFAULT 0;")?;

    conn.execute_batch(
        "
        UPDATE accounts SET opening_balance = balance - (
            COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'income' AND account_id = accounts.id), 0)
            - COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'expense' AND account_id = accounts.id), 0)
            - COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'transfer' AND account_id = accounts.id), 0)
            + COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'transfer' AND transfer_account_id = accounts.id), 0)
        );
        ",
    )
}

/// Seed first-run defaults: a profile, a handful of flat categories, and a
/// default account.
///
/// Each step is independently idempotent (gated on its own table being
/// empty), so a database created before the default-account seed existed
/// still gains an account on the next launch. Without at least one account,
/// every transaction insert would fail the `account_id` foreign key.
pub fn seed_defaults(conn: &Connection) -> rusqlite::Result<()> {
    let profile_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM app_profiles;", [], |row| row.get(0))?;

    let profile_id: i64 = if profile_count == 0 {
        // Use a fixed ISO-8601-ish timestamp source via SQLite's datetime().
        conn.execute(
            "INSERT INTO app_profiles (profile_name, created_at) VALUES (?1, datetime('now'));",
            ["Personal"],
        )?;
        let pid = conn.last_insert_rowid();

        // (name, color_code, type)
        let categories: [(&str, &str, &str); 5] = [
            ("Dining Out", "#E57373", "expense"),
            ("Groceries", "#81C784", "expense"),
            ("Rent", "#64B5F6", "expense"),
            ("Transport", "#FFB74D", "expense"),
            ("Salary", "#4DB6AC", "income"),
        ];
        let mut dining_id: Option<i64> = None;
        {
            let mut stmt = conn.prepare(
                "INSERT INTO categories (profile_id, name, color_code, type) VALUES (?1, ?2, ?3, ?4);",
            )?;
            for (name, color, kind) in categories {
                stmt.execute(rusqlite::params![pid, name, color, kind])?;
                if name == "Dining Out" {
                    dining_id = Some(conn.last_insert_rowid());
                }
            }
        }

        // A couple of example sub-categories under Dining Out, so the one-level
        // nesting is discoverable on first run.
        if let Some(parent) = dining_id {
            let subs: [(&str, &str); 3] =
                [("Restaurants", "#EF9A9A"), ("Coffee", "#F48FB1"), ("Takeout", "#FFAB91")];
            let mut sub_stmt = conn.prepare(
                "INSERT INTO categories (profile_id, name, color_code, type, parent_id) \
                 VALUES (?1, ?2, ?3, 'expense', ?4);",
            )?;
            for (name, color) in subs {
                sub_stmt.execute(rusqlite::params![pid, name, color, parent])?;
            }
        }
        pid
    } else {
        conn.query_row("SELECT id FROM app_profiles ORDER BY id LIMIT 1;", [], |row| {
            row.get(0)
        })?
    };

    // Every transaction references an account — guarantee at least one exists.
    let account_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM accounts;", [], |row| row.get(0))?;
    if account_count == 0 {
        conn.execute(
            "INSERT INTO accounts (profile_id, name, balance, currency, type, opening_balance) \
             VALUES (?1, 'Cash', 0, 'USD', 'cash', 0);",
            rusqlite::params![profile_id],
        )?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Backup & restore helpers — plain functions over file paths, unit-testable.
// ---------------------------------------------------------------------------

/// Run `PRAGMA integrity_check` against the SQLite file at `path`.
///
/// Returns `Ok(())` only when SQLite reports the single row `"ok"`. A missing
/// file, a non-SQLite file, or a corrupt database all surface as an `Err` with
/// a human-readable message (so the frontend can show it in a toast).
pub fn check_integrity(path: &Path) -> Result<(), String> {
    let conn =
        Connection::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let result: String = conn
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .map_err(|e| format!("not a valid database: {e}"))?;
    if result != "ok" {
        return Err(format!("integrity check failed: {result}"));
    }
    Ok(())
}

/// Copy the database file at `src` to `dst`, then verify the copy passes
/// [`check_integrity`]. A copy that fails verification is removed so a corrupt
/// backup is never left behind.
///
/// The caller is responsible for holding the DB lock (so no write is in flight)
/// while this runs; with WAL disabled an idle connection leaves a consistent
/// on-disk file that is safe to copy.
pub fn export_to(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::copy(src, dst)
        .map_err(|e| format!("failed to write backup to {}: {e}", dst.display()))?;
    if let Err(e) = check_integrity(dst) {
        let _ = std::fs::remove_file(dst);
        return Err(format!("backup verification failed: {e}"));
    }
    Ok(())
}

/// Prefix of the automatic safety copies written before a restore.
pub const PRE_RESTORE_PREFIX: &str = "budget-pre-restore-";

/// Delete the oldest `budget-pre-restore-*.db` safety copies in `dir`, keeping
/// only the `keep` most recent, so they don't accumulate without bound.
///
/// The filenames embed a Unix-seconds timestamp, which is fixed-width for the
/// foreseeable future, so a lexical sort is chronological. Best-effort: an
/// unreadable directory or a failed delete is reported but never aborts a
/// restore (the caller ignores the result).
pub fn prune_pre_restore_backups(dir: &Path, keep: usize) -> Result<(), String> {
    let mut backups: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read backup directory {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(PRE_RESTORE_PREFIX) && n.ends_with(".db"))
        })
        .collect();
    backups.sort();

    if backups.len() > keep {
        for old in &backups[..backups.len() - keep] {
            let _ = std::fs::remove_file(old);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1;",
            [name],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            > 0
    }

    /// A fresh, unique temp directory for file-based tests, removed by the
    /// returned guard on drop so the suite leaves nothing behind.
    struct TmpDir(PathBuf);
    impl TmpDir {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let mut p = std::env::temp_dir();
            p.push(format!("budetau-test-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            TmpDir(p)
        }
        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn init_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();

        for table in [
            "app_profiles",
            "accounts",
            "categories",
            "budgets",
            "transactions",
        ] {
            assert!(table_exists(&conn, table), "missing table: {table}");
        }
    }

    #[test]
    fn init_sets_user_version() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn init_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        // Running again must not error (e.g. CREATE TABLE IF NOT EXISTS).
        init(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn foreign_keys_pragma_is_on() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();

        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1, "foreign_keys pragma should be ON");
    }

    #[test]
    fn seed_creates_one_profile_and_categories() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        seed_defaults(&conn).unwrap();

        let profiles: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_profiles;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(profiles, 1);

        let categories: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories;", [], |row| row.get(0))
            .unwrap();
        assert!(categories > 0, "expected seeded categories");
    }

    #[test]
    fn seed_creates_a_default_account() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        seed_defaults(&conn).unwrap();

        let accounts: i64 = conn
            .query_row("SELECT COUNT(*) FROM accounts;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(accounts, 1, "expected one default account so transactions can be inserted");
    }

    #[test]
    fn seed_backfills_account_for_legacy_db() {
        // Simulate a DB seeded before the default-account step existed:
        // a profile and categories, but zero accounts.
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn.execute(
            "INSERT INTO app_profiles (profile_name, created_at) VALUES ('Personal', datetime('now'));",
            [],
        )
        .unwrap();

        seed_defaults(&conn).unwrap();

        let accounts: i64 = conn
            .query_row("SELECT COUNT(*) FROM accounts;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(accounts, 1, "existing profile should still get a default account");
        let profiles: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_profiles;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(profiles, 1, "must not create a second profile");
    }

    #[test]
    fn seed_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        seed_defaults(&conn).unwrap();
        seed_defaults(&conn).unwrap();

        let profiles: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_profiles;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(profiles, 1, "seeding twice must not duplicate profiles");

        let categories: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories;", [], |row| row.get(0))
            .unwrap();
        // 5 top-level + 3 example sub-categories under Dining Out.
        assert_eq!(categories, 8, "seeding twice must not duplicate categories");

        let accounts: i64 = conn
            .query_row("SELECT COUNT(*) FROM accounts;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(accounts, 1, "seeding twice must not duplicate the default account");
    }

    #[test]
    fn export_creates_a_verified_copy() {
        let dir = TmpDir::new();
        let src = dir.join("budget.db");
        let dst = dir.join("backup.db");

        let conn = Connection::open(&src).unwrap();
        init(&conn).unwrap();
        seed_defaults(&conn).unwrap();
        drop(conn);

        export_to(&src, &dst).unwrap();
        assert!(dst.exists(), "backup file should exist");
        // The copy is a real, intact database at the current schema version.
        check_integrity(&dst).unwrap();
        let restored = Connection::open(&dst).unwrap();
        let version: i64 = restored
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn check_integrity_rejects_a_non_database_file() {
        let dir = TmpDir::new();
        let bogus = dir.join("not.db");
        std::fs::write(&bogus, b"this is plainly not a sqlite database").unwrap();
        assert!(
            check_integrity(&bogus).is_err(),
            "a non-SQLite file must fail the integrity check"
        );
    }

    #[test]
    fn export_then_reinit_restores_to_current_schema() {
        // A backup taken from an older schema is brought current by `init` on
        // restore — exercise that the migration path runs against a copy.
        let dir = TmpDir::new();
        let old = dir.join("old.db");

        let conn = Connection::open(&old).unwrap();
        // Simulate a v1-only database (no alert_threshold / parent_id yet).
        migrate_v1(&conn).unwrap();
        conn.execute_batch("PRAGMA user_version = 1;").unwrap();
        drop(conn);

        let copy = dir.join("copy.db");
        export_to(&old, &copy).unwrap();

        // Restoring re-runs init, which migrates the copy forward.
        let restored = Connection::open(&copy).unwrap();
        init(&restored).unwrap();
        let version: i64 = restored
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrate_v4_backfills_opening_balance_from_transaction_history() {
        // Build a v3 database by hand (pre-opening_balance), matching how a
        // real upgrade would look: an account whose `balance` already
        // reflects some transaction history, but with no independent record
        // of what its starting balance was.
        let conn = Connection::open_in_memory().unwrap();
        migrate_v1(&conn).unwrap();
        migrate_v2(&conn).unwrap();
        migrate_v3(&conn).unwrap();
        conn.execute_batch("PRAGMA user_version = 3;").unwrap();

        conn.execute(
            "INSERT INTO app_profiles (profile_name, created_at) VALUES ('Personal', datetime('now'));",
            [],
        )
        .unwrap();
        let profile_id = conn.last_insert_rowid();

        // Started at $1,000.00, then a $25.00 expense and a $70.00 income:
        // current balance = 100_000 - 2_500 + 7_000 = 104_500.
        conn.execute(
            "INSERT INTO accounts (profile_id, name, balance, currency, type) \
             VALUES (?1, 'Checking', 104500, 'USD', 'checking');",
            rusqlite::params![profile_id],
        )
        .unwrap();
        let account_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO transactions (account_id, amount, date, type) \
             VALUES (?1, 2500, '2026-06-25', 'expense');",
            rusqlite::params![account_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transactions (account_id, amount, date, type) \
             VALUES (?1, 7000, '2026-06-25', 'income');",
            rusqlite::params![account_id],
        )
        .unwrap();

        // `init` applies migration v4, which adds and backfills
        // `opening_balance`.
        init(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);

        let opening: i64 = conn
            .query_row(
                "SELECT opening_balance FROM accounts WHERE id = ?1;",
                [account_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(opening, 100_000, "backfilled opening balance should recover the starting balance");

        // A recompute afterwards must leave the balance unchanged — no drift
        // was introduced by the backfill.
        let balance: i64 = conn
            .query_row("SELECT balance FROM accounts WHERE id = ?1;", [account_id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(balance, 104_500);
    }

    #[test]
    fn migrate_v4_is_a_no_op_on_a_fresh_database_with_no_accounts() {
        // On a brand-new DB there are no accounts yet at migration time, so
        // the backfill UPDATE simply touches zero rows.
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);

        let accounts: i64 = conn
            .query_row("SELECT COUNT(*) FROM accounts;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(accounts, 0);
    }

    #[test]
    fn prune_keeps_only_the_newest_pre_restore_backups() {
        let dir = TmpDir::new();

        // Six safety copies with ascending (lexically sortable) timestamps, plus
        // two unrelated files that must be left alone.
        for stamp in ["1000000001", "1000000002", "1000000003", "1000000004", "1000000005", "1000000006"] {
            std::fs::write(dir.join(&format!("{PRE_RESTORE_PREFIX}{stamp}.db")), b"x").unwrap();
        }
        std::fs::write(dir.join("budget.db"), b"live").unwrap();
        std::fs::write(dir.join("my-manual-backup.db"), b"keep").unwrap();

        prune_pre_restore_backups(&dir.0, 2).unwrap();

        // Only the two newest safety copies survive.
        assert!(!dir.join(&format!("{PRE_RESTORE_PREFIX}1000000004.db")).exists());
        assert!(dir.join(&format!("{PRE_RESTORE_PREFIX}1000000005.db")).exists());
        assert!(dir.join(&format!("{PRE_RESTORE_PREFIX}1000000006.db")).exists());
        // Unrelated files are untouched.
        assert!(dir.join("budget.db").exists());
        assert!(dir.join("my-manual-backup.db").exists());
    }

    #[test]
    fn prune_is_a_no_op_when_under_the_keep_limit() {
        let dir = TmpDir::new();
        std::fs::write(dir.join(&format!("{PRE_RESTORE_PREFIX}1000000001.db")), b"x").unwrap();
        prune_pre_restore_backups(&dir.0, 5).unwrap();
        assert!(dir.join(&format!("{PRE_RESTORE_PREFIX}1000000001.db")).exists());
    }
}
