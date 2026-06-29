use std::path::PathBuf;

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
const CURRENT_SCHEMA_VERSION: i64 = 3;

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

    debug_assert_eq!(
        conn.query_row("PRAGMA user_version;", [], |row| row.get::<_, i64>(0))?,
        CURRENT_SCHEMA_VERSION
    );
    Ok(())
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

/// The v1 schema. Money is always stored as INTEGER minor units (cents);
/// never REAL, to avoid floating-point rounding errors.
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
            "INSERT INTO accounts (profile_id, name, balance, currency, type) \
             VALUES (?1, 'Cash', 0, 'USD', 'cash');",
            rusqlite::params![profile_id],
        )?;
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
}
