use std::sync::Mutex;

use rusqlite::Connection;
use tauri::Manager;

mod basic;
mod commands;
mod db;

/// Tauri managed state holding the single SQLite connection.
///
/// CRUD commands access this via `tauri::State<'_, DbState>` and lock the
/// inner `Mutex<Connection>` to run queries.
pub struct DbState(pub Mutex<Connection>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path =
                db::resolve_db_path(app.handle()).map_err(Box::<dyn std::error::Error>::from)?;
            let mut conn = Connection::open(&path)?;
            db::init(&conn)?;
            basic::initialize(&mut conn)?;

            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            basic::load_basic_budget,
            basic::save_basic_budget,
            basic::write_basic_backup,
            basic::read_basic_backup,
            commands::get_db_path,
            commands::list_accounts,
            commands::list_categories,
            commands::list_budgets,
            commands::list_transactions,
            commands::create_account,
            commands::update_account,
            commands::delete_account,
            commands::recompute_balances,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            commands::create_budget,
            commands::update_budget,
            commands::delete_budget,
            commands::create_transaction,
            commands::update_transaction,
            commands::delete_transaction,
            commands::get_budget_month,
            commands::set_assigned,
            commands::list_recurring_rules,
            commands::create_recurring_rule,
            commands::update_recurring_rule,
            commands::delete_recurring_rule,
            commands::post_due_recurring_cmd,
            commands::export_backup,
            commands::restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
