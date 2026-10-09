use rusqlite::{Connection, OptionalExtension};
use crate::DbState;

// One-time transition to the basic app. The user requested an empty history.
// The reset and marker are atomic; subsequent launches keep the saved budget.
pub fn initialize(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS basic_budget (id INTEGER PRIMARY KEY CHECK(id = 1), payload TEXT NOT NULL);")?;
    let exists: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM basic_budget WHERE id = 1)", [], |r| r.get(0))?;
    if !exists {
        let tx = conn.transaction()?;
        for table in ["transactions", "recurring_rules", "category_months", "budgets", "accounts", "categories", "app_profiles"] {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
        tx.execute("INSERT INTO basic_budget (id, payload) VALUES (1, 'null')", [])?;
        tx.commit()?;
    }
    Ok(())
}

fn validate_payload(payload: &str) -> Result<(), String> {
    if payload.len() > 20_000_000 { return Err("Budget backup is too large".into()); }
    let value: serde_json::Value = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(1)
        || !value.get("months").is_some_and(|v| v.is_object())
        || !value.get("entries").is_some_and(|v| v.is_array()) {
        return Err("Not a Budetau basic backup".into());
    }
    Ok(())
}

#[tauri::command]
pub fn load_basic_budget(state: tauri::State<'_, DbState>) -> Result<Option<String>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.query_row("SELECT payload FROM basic_budget WHERE id = 1", [], |r| r.get(0)).optional().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_basic_budget(state: tauri::State<'_, DbState>, payload: String) -> Result<(), String> {
    validate_payload(&payload)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO basic_budget (id, payload) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET payload = excluded.payload", [&payload]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn write_basic_backup(path: String, payload: String) -> Result<(), String> {
    validate_payload(&payload)?;
    if std::path::Path::new(&path).extension().and_then(|x| x.to_str()) != Some("json") { return Err("Choose a .json backup file".into()); }
    std::fs::write(path, payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_basic_backup(path: String) -> Result<String, String> {
    if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 20_000_000 { return Err("Budget backup is too large".into()); }
    let payload = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    validate_payload(&payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_launch_clears_legacy_data_and_next_launch_keeps_basic_data() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init(&conn).unwrap();
        crate::db::seed_defaults(&conn).unwrap();
        initialize(&mut conn).unwrap();
        let count: i64 = conn.query_row("SELECT count(*) FROM accounts", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
        conn.execute("UPDATE basic_budget SET payload = ?1", [r#"{"version":1,"months":{},"entries":[]}"#]).unwrap();
        initialize(&mut conn).unwrap();
        let payload: String = conn.query_row("SELECT payload FROM basic_budget", [], |r| r.get(0)).unwrap();
        assert!(payload.contains("version"));
    }
    #[test]
    fn invalid_backups_are_rejected() {
        assert!(validate_payload("null").is_err());
        assert!(validate_payload(r#"{"version":2,"months":{},"entries":[]}"#).is_err());
        assert!(validate_payload(r#"{"version":1,"months":{},"entries":[]}"#).is_ok());
    }
}
