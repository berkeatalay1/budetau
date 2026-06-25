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
const CURRENT_SCHEMA_VERSION: i64 = 1;

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

    if version < 1 {
        migrate_v1(conn)?;
        // `user_version` does not accept a bound parameter, so format it in.
        // The value is an internal constant, not user input.
        conn.execute_batch(&format!("PRAGMA user_version = {CURRENT_SCHEMA_VERSION};"))?;
    }

    Ok(())
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

/// Seed first-run defaults: one profile plus a handful of flat categories.
///
/// Idempotent: if any profile already exists, this is a no-op.
pub fn seed_defaults(conn: &Connection) -> rusqlite::Result<()> {
    let profile_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM app_profiles;", [], |row| row.get(0))?;
    if profile_count > 0 {
        return Ok(());
    }

    // Use a fixed ISO-8601-ish timestamp source via SQLite's datetime().
    conn.execute(
        "INSERT INTO app_profiles (profile_name, created_at) VALUES (?1, datetime('now'));",
        ["Personal"],
    )?;
    let profile_id = conn.last_insert_rowid();

    // (name, color_code, type)
    let categories: [(&str, &str, &str); 5] = [
        ("Dining Out", "#E57373", "expense"),
        ("Groceries", "#81C784", "expense"),
        ("Rent", "#64B5F6", "expense"),
        ("Transport", "#FFB74D", "expense"),
        ("Salary", "#4DB6AC", "income"),
    ];

    let mut stmt = conn.prepare(
        "INSERT INTO categories (profile_id, name, color_code, type) VALUES (?1, ?2, ?3, ?4);",
    )?;
    for (name, color, kind) in categories {
        stmt.execute(rusqlite::params![profile_id, name, color, kind])?;
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
        assert_eq!(categories, 5, "seeding twice must not duplicate categories");
    }
}
