//! Tauri IPC command layer.
//!
//! The pattern here separates *DB logic* from *Tauri glue*:
//!
//! * Plain functions take a `&Connection` (or `&Transaction`) and return
//!   `rusqlite::Result<T>`. These hold all the SQL and are unit-testable
//!   against an in-memory database.
//! * `#[tauri::command]` wrappers lock the managed [`DbState`] connection,
//!   call the plain function, and map any error to a `String` so it crosses
//!   the IPC boundary cleanly.
//!
//! All serde structs use `rename_all = "camelCase"` so the Vue frontend
//! receives camelCase field names automatically.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::db;
use crate::DbState;

// ---------------------------------------------------------------------------
// Data model — mirrors the v1 SQLite schema. Money is INTEGER cents.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: i64,
    pub profile_id: i64,
    pub name: String,
    pub balance: i64,
    pub currency: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub profile_id: i64,
    pub name: String,
    pub color_code: Option<String>,
    pub r#type: String,
    /// Parent category id, or None for a top-level category.
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    pub id: i64,
    pub profile_id: i64,
    pub category_id: i64,
    pub period: String,
    pub amount: i64,
    /// Fraction of `amount` at which the budget is "near" its limit (0.0–1.0).
    pub alert_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: i64,
    pub account_id: i64,
    pub amount: i64,
    pub category_id: Option<i64>,
    pub date: String,
    pub note: Option<String>,
    pub r#type: String,
    pub transfer_account_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// Create payloads (no `id` — assigned by SQLite).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAccount {
    pub profile_id: i64,
    pub name: String,
    #[serde(default)]
    pub balance: i64,
    pub currency: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCategory {
    pub profile_id: i64,
    pub name: String,
    pub color_code: Option<String>,
    pub r#type: String,
    #[serde(default)]
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBudget {
    pub profile_id: i64,
    pub category_id: i64,
    pub period: String,
    pub amount: i64,
    /// Defaults to 0.9 (90%) when omitted by the frontend.
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
}

fn default_alert_threshold() -> f64 {
    0.9
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTransaction {
    pub account_id: i64,
    pub amount: i64,
    pub category_id: Option<i64>,
    pub date: String,
    pub note: Option<String>,
    pub r#type: String,
    pub transfer_account_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// DB logic — plain functions over a `&Connection`. Unit-testable.
// ---------------------------------------------------------------------------

fn map_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get("id")?,
        profile_id: row.get("profile_id")?,
        name: row.get("name")?,
        balance: row.get("balance")?,
        currency: row.get("currency")?,
        r#type: row.get("type")?,
    })
}

fn map_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get("id")?,
        profile_id: row.get("profile_id")?,
        name: row.get("name")?,
        color_code: row.get("color_code")?,
        r#type: row.get("type")?,
        parent_id: row.get("parent_id")?,
    })
}

fn map_budget(row: &rusqlite::Row<'_>) -> rusqlite::Result<Budget> {
    Ok(Budget {
        id: row.get("id")?,
        profile_id: row.get("profile_id")?,
        category_id: row.get("category_id")?,
        period: row.get("period")?,
        amount: row.get("amount")?,
        alert_threshold: row.get("alert_threshold")?,
    })
}

fn map_transaction(row: &rusqlite::Row<'_>) -> rusqlite::Result<Transaction> {
    Ok(Transaction {
        id: row.get("id")?,
        account_id: row.get("account_id")?,
        amount: row.get("amount")?,
        category_id: row.get("category_id")?,
        date: row.get("date")?,
        note: row.get("note")?,
        r#type: row.get("type")?,
        transfer_account_id: row.get("transfer_account_id")?,
    })
}

fn select_accounts(conn: &Connection) -> rusqlite::Result<Vec<Account>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, name, balance, currency, type FROM accounts ORDER BY id;",
    )?;
    let rows = stmt.query_map([], map_account)?;
    rows.collect()
}

fn select_categories(conn: &Connection) -> rusqlite::Result<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, name, color_code, type, parent_id FROM categories ORDER BY id;",
    )?;
    let rows = stmt.query_map([], map_category)?;
    rows.collect()
}

fn select_budgets(conn: &Connection) -> rusqlite::Result<Vec<Budget>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, category_id, period, amount, alert_threshold \
         FROM budgets ORDER BY id;",
    )?;
    let rows = stmt.query_map([], map_budget)?;
    rows.collect()
}

fn select_transactions(conn: &Connection) -> rusqlite::Result<Vec<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, amount, category_id, date, note, type, transfer_account_id \
         FROM transactions ORDER BY date DESC, id DESC;",
    )?;
    let rows = stmt.query_map([], map_transaction)?;
    rows.collect()
}

fn fetch_account(conn: &Connection, id: i64) -> rusqlite::Result<Account> {
    conn.query_row(
        "SELECT id, profile_id, name, balance, currency, type FROM accounts WHERE id = ?1;",
        params![id],
        map_account,
    )
}

fn fetch_category(conn: &Connection, id: i64) -> rusqlite::Result<Category> {
    conn.query_row(
        "SELECT id, profile_id, name, color_code, type, parent_id FROM categories WHERE id = ?1;",
        params![id],
        map_category,
    )
}

fn fetch_budget(conn: &Connection, id: i64) -> rusqlite::Result<Budget> {
    conn.query_row(
        "SELECT id, profile_id, category_id, period, amount, alert_threshold \
         FROM budgets WHERE id = ?1;",
        params![id],
        map_budget,
    )
}

fn fetch_transaction(conn: &Connection, id: i64) -> rusqlite::Result<Transaction> {
    conn.query_row(
        "SELECT id, account_id, amount, category_id, date, note, type, transfer_account_id \
         FROM transactions WHERE id = ?1;",
        params![id],
        map_transaction,
    )
}

fn insert_account(conn: &Connection, new: &NewAccount) -> rusqlite::Result<Account> {
    conn.execute(
        "INSERT INTO accounts (profile_id, name, balance, currency, type) \
         VALUES (?1, ?2, ?3, ?4, ?5);",
        params![new.profile_id, new.name, new.balance, new.currency, new.r#type],
    )?;
    fetch_account(conn, conn.last_insert_rowid())
}

fn insert_category(conn: &Connection, new: &NewCategory) -> rusqlite::Result<Category> {
    conn.execute(
        "INSERT INTO categories (profile_id, name, color_code, type, parent_id) \
         VALUES (?1, ?2, ?3, ?4, ?5);",
        params![new.profile_id, new.name, new.color_code, new.r#type, new.parent_id],
    )?;
    fetch_category(conn, conn.last_insert_rowid())
}

fn update_category_logic(conn: &Connection, c: &Category) -> rusqlite::Result<Category> {
    conn.execute(
        "UPDATE categories SET name = ?1, color_code = ?2, type = ?3, parent_id = ?4 WHERE id = ?5;",
        params![c.name, c.color_code, c.r#type, c.parent_id, c.id],
    )?;
    fetch_category(conn, c.id)
}

/// Delete a category, refusing if it still has sub-categories or any
/// transaction references it (which would orphan rows / break the FK).
fn delete_category_logic(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let children: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE parent_id = ?1;",
        params![id],
        |row| row.get(0),
    )?;
    if children > 0 {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "category has {children} sub-categories; remove or reparent them first"
        )));
    }
    let txns: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transactions WHERE category_id = ?1;",
        params![id],
        |row| row.get(0),
    )?;
    if txns > 0 {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "category is used by {txns} transaction(s); recategorize them first"
        )));
    }
    conn.execute("DELETE FROM categories WHERE id = ?1;", params![id])?;
    Ok(())
}

fn insert_budget(conn: &Connection, new: &NewBudget) -> rusqlite::Result<Budget> {
    conn.execute(
        "INSERT INTO budgets (profile_id, category_id, period, amount, alert_threshold) \
         VALUES (?1, ?2, ?3, ?4, ?5);",
        params![
            new.profile_id,
            new.category_id,
            new.period,
            new.amount,
            new.alert_threshold
        ],
    )?;
    fetch_budget(conn, conn.last_insert_rowid())
}

fn update_budget_logic(conn: &Connection, b: &Budget) -> rusqlite::Result<Budget> {
    conn.execute(
        "UPDATE budgets SET category_id = ?1, period = ?2, amount = ?3, alert_threshold = ?4 \
         WHERE id = ?5;",
        params![b.category_id, b.period, b.amount, b.alert_threshold, b.id],
    )?;
    fetch_budget(conn, b.id)
}

fn delete_budget_logic(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM budgets WHERE id = ?1;", params![id])?;
    Ok(())
}

fn update_account_logic(conn: &Connection, a: &Account) -> rusqlite::Result<Account> {
    conn.execute(
        "UPDATE accounts SET name = ?1, currency = ?2, type = ?3 WHERE id = ?4;",
        params![a.name, a.currency, a.r#type, a.id],
    )?;
    fetch_account(conn, a.id)
}

/// Delete an account, refusing if any transaction still references it (either
/// as its account or as a transfer destination) — that would orphan history
/// and break balances.
fn delete_account_logic(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    let referencing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transactions WHERE account_id = ?1 OR transfer_account_id = ?1;",
        params![id],
        |row| row.get(0),
    )?;
    if referencing > 0 {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "account has {referencing} transaction(s); reassign or delete them first"
        )));
    }
    conn.execute("DELETE FROM accounts WHERE id = ?1;", params![id])?;
    Ok(())
}

/// Apply (`dir = 1`) or reverse (`dir = -1`) a transaction's effect on
/// account balance(s). Centralises the per-`type` rules so insert, update,
/// and delete stay consistent. `amount` is a positive magnitude in cents;
/// the sign comes from `type`.
///
/// * `expense`  — subtract from `account_id`.
/// * `income`   — add to `account_id`.
/// * `transfer` — subtract from `account_id`, add to `transfer_account_id`.
fn adjust_balances(
    tx: &rusqlite::Transaction<'_>,
    ttype: &str,
    amount: i64,
    account_id: i64,
    transfer_account_id: Option<i64>,
    dir: i64,
) -> rusqlite::Result<()> {
    let amt = amount * dir;
    match ttype {
        "expense" => {
            tx.execute(
                "UPDATE accounts SET balance = balance - ?1 WHERE id = ?2;",
                params![amt, account_id],
            )?;
        }
        "income" => {
            tx.execute(
                "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2;",
                params![amt, account_id],
            )?;
        }
        "transfer" => {
            let dest = transfer_account_id.ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    "transfer transaction requires transfer_account_id".to_string(),
                )
            })?;
            tx.execute(
                "UPDATE accounts SET balance = balance - ?1 WHERE id = ?2;",
                params![amt, account_id],
            )?;
            tx.execute(
                "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2;",
                params![amt, dest],
            )?;
        }
        other => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unknown transaction type: {other}"
            )));
        }
    }
    Ok(())
}

/// Insert a transaction and atomically adjust account balance(s).
///
/// CRITICAL: the row insert and the balance update happen inside one rusqlite
/// transaction, committed together, so a balance can never drift out of sync
/// with its transaction history.
fn insert_transaction(conn: &mut Connection, new: &NewTransaction) -> rusqlite::Result<Transaction> {
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO transactions \
         (account_id, amount, category_id, date, note, type, transfer_account_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
        params![
            new.account_id,
            new.amount,
            new.category_id,
            new.date,
            new.note,
            new.r#type,
            new.transfer_account_id
        ],
    )?;
    let id = tx.last_insert_rowid();

    adjust_balances(
        &tx,
        &new.r#type,
        new.amount,
        new.account_id,
        new.transfer_account_id,
        1,
    )?;

    let created = fetch_transaction(&tx, id)?;
    tx.commit()?;
    Ok(created)
}

/// Update a transaction, atomically reversing its old balance effect and
/// applying the new one in a single rusqlite transaction (so balances never
/// drift, even when the amount, type, or accounts change).
fn update_transaction_logic(
    conn: &mut Connection,
    t: &Transaction,
) -> rusqlite::Result<Transaction> {
    let tx = conn.transaction()?;

    let old = fetch_transaction(&tx, t.id)?;
    adjust_balances(
        &tx,
        &old.r#type,
        old.amount,
        old.account_id,
        old.transfer_account_id,
        -1,
    )?;

    tx.execute(
        "UPDATE transactions SET \
         account_id = ?1, amount = ?2, category_id = ?3, date = ?4, note = ?5, \
         type = ?6, transfer_account_id = ?7 WHERE id = ?8;",
        params![
            t.account_id,
            t.amount,
            t.category_id,
            t.date,
            t.note,
            t.r#type,
            t.transfer_account_id,
            t.id
        ],
    )?;

    adjust_balances(
        &tx,
        &t.r#type,
        t.amount,
        t.account_id,
        t.transfer_account_id,
        1,
    )?;

    let updated = fetch_transaction(&tx, t.id)?;
    tx.commit()?;
    Ok(updated)
}

/// Delete a transaction, atomically reversing its balance effect first.
fn delete_transaction_logic(conn: &mut Connection, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;

    let old = fetch_transaction(&tx, id)?;
    adjust_balances(
        &tx,
        &old.r#type,
        old.amount,
        old.account_id,
        old.transfer_account_id,
        -1,
    )?;
    tx.execute("DELETE FROM transactions WHERE id = ?1;", params![id])?;

    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri command wrappers — lock state, call logic, map errors to String.
// ---------------------------------------------------------------------------

/// Return the resolved on-disk SQLite database path as a string.
#[tauri::command]
pub fn get_db_path(app: AppHandle) -> Result<String, String> {
    let path = db::resolve_db_path(&app)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn list_accounts(state: tauri::State<'_, DbState>) -> Result<Vec<Account>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    select_accounts(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_categories(state: tauri::State<'_, DbState>) -> Result<Vec<Category>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    select_categories(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_budgets(state: tauri::State<'_, DbState>) -> Result<Vec<Budget>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    select_budgets(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_transactions(state: tauri::State<'_, DbState>) -> Result<Vec<Transaction>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    select_transactions(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_account(
    state: tauri::State<'_, DbState>,
    account: NewAccount,
) -> Result<Account, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    insert_account(&conn, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_account(
    state: tauri::State<'_, DbState>,
    account: Account,
) -> Result<Account, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_account_logic(&conn, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_account(state: tauri::State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    delete_account_logic(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_category(
    state: tauri::State<'_, DbState>,
    category: NewCategory,
) -> Result<Category, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    insert_category(&conn, &category).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(
    state: tauri::State<'_, DbState>,
    category: Category,
) -> Result<Category, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_category_logic(&conn, &category).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(state: tauri::State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    delete_category_logic(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_budget(
    state: tauri::State<'_, DbState>,
    budget: NewBudget,
) -> Result<Budget, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    insert_budget(&conn, &budget).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_budget(state: tauri::State<'_, DbState>, budget: Budget) -> Result<Budget, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_budget_logic(&conn, &budget).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_budget(state: tauri::State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    delete_budget_logic(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_transaction(
    state: tauri::State<'_, DbState>,
    transaction: NewTransaction,
) -> Result<Transaction, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    insert_transaction(&mut conn, &transaction).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_transaction(
    state: tauri::State<'_, DbState>,
    transaction: Transaction,
) -> Result<Transaction, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    update_transaction_logic(&mut conn, &transaction).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_transaction(state: tauri::State<'_, DbState>, id: i64) -> Result<(), String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    delete_transaction_logic(&mut conn, id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Tests — exercise the plain DB logic against an in-memory database.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an in-memory DB with the schema, one profile, and a couple of
    /// accounts. Returns the connection plus the two account ids.
    fn setup() -> (Connection, i64, i64) {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO app_profiles (profile_name, created_at) VALUES ('Test', datetime('now'));",
            [],
        )
        .unwrap();
        let profile_id = conn.last_insert_rowid();

        let checking = insert_account(
            &conn,
            &NewAccount {
                profile_id,
                name: "Checking".into(),
                balance: 100_000, // $1,000.00
                currency: "USD".into(),
                r#type: "checking".into(),
            },
        )
        .unwrap();

        let savings = insert_account(
            &conn,
            &NewAccount {
                profile_id,
                name: "Savings".into(),
                balance: 50_000, // $500.00
                currency: "USD".into(),
                r#type: "savings".into(),
            },
        )
        .unwrap();

        (conn, checking.id, savings.id)
    }

    fn balance_of(conn: &Connection, id: i64) -> i64 {
        fetch_account(conn, id).unwrap().balance
    }

    #[test]
    fn expense_decreases_account_balance() {
        let (mut conn, checking, _savings) = setup();

        insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 2_500, // $25.00
                category_id: None,
                date: "2026-06-25".into(),
                note: Some("Lunch".into()),
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();

        assert_eq!(balance_of(&conn, checking), 100_000 - 2_500);
    }

    #[test]
    fn income_increases_account_balance() {
        let (mut conn, checking, _savings) = setup();

        insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 7_000, // $70.00
                category_id: None,
                date: "2026-06-25".into(),
                note: Some("Refund".into()),
                r#type: "income".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();

        assert_eq!(balance_of(&conn, checking), 100_000 + 7_000);
    }

    #[test]
    fn transfer_moves_cents_between_accounts() {
        let (mut conn, checking, savings) = setup();

        insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 30_000, // $300.00
                category_id: None,
                date: "2026-06-25".into(),
                note: Some("To savings".into()),
                r#type: "transfer".into(),
                transfer_account_id: Some(savings),
            },
        )
        .unwrap();

        assert_eq!(balance_of(&conn, checking), 100_000 - 30_000);
        assert_eq!(balance_of(&conn, savings), 50_000 + 30_000);
    }

    #[test]
    fn transfer_without_destination_errors_and_rolls_back() {
        let (mut conn, checking, _savings) = setup();

        let result = insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 10_000,
                category_id: None,
                date: "2026-06-25".into(),
                note: None,
                r#type: "transfer".into(),
                transfer_account_id: None,
            },
        );

        assert!(result.is_err(), "transfer without destination must fail");
        // Rolled back: balance unchanged and no transaction row persisted.
        assert_eq!(balance_of(&conn, checking), 100_000);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn list_and_create_roundtrip() {
        let (conn, _checking, _savings) = setup();

        let accounts = select_accounts(&conn).unwrap();
        assert_eq!(accounts.len(), 2);

        let cat = insert_category(
            &conn,
            &NewCategory {
                profile_id: accounts[0].profile_id,
                name: "Utilities".into(),
                color_code: Some("#888888".into()),
                r#type: "expense".into(),
                parent_id: None,
            },
        )
        .unwrap();
        assert_eq!(cat.name, "Utilities");

        let budget = insert_budget(
            &conn,
            &NewBudget {
                profile_id: accounts[0].profile_id,
                category_id: cat.id,
                period: "monthly".into(),
                amount: 12_000,
                alert_threshold: 0.9,
            },
        )
        .unwrap();
        assert_eq!(budget.amount, 12_000);
        assert_eq!(budget.alert_threshold, 0.9);
        assert_eq!(select_budgets(&conn).unwrap().len(), 1);
    }

    #[test]
    fn delete_reverses_balance_effect() {
        let (mut conn, checking, _savings) = setup();
        let t = insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 2_500,
                category_id: None,
                date: "2026-06-25".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();
        assert_eq!(balance_of(&conn, checking), 100_000 - 2_500);

        delete_transaction_logic(&mut conn, t.id).unwrap();
        // Balance restored and row gone.
        assert_eq!(balance_of(&conn, checking), 100_000);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn update_amount_reapplies_balance_delta() {
        let (mut conn, checking, _savings) = setup();
        let mut t = insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 2_500,
                category_id: None,
                date: "2026-06-25".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();
        assert_eq!(balance_of(&conn, checking), 97_500);

        // Bump the expense from $25 to $40: balance should drop a further $15.
        t.amount = 4_000;
        update_transaction_logic(&mut conn, &t).unwrap();
        assert_eq!(balance_of(&conn, checking), 100_000 - 4_000);
    }

    #[test]
    fn update_changing_type_expense_to_income_flips_sign() {
        let (mut conn, checking, _savings) = setup();
        let mut t = insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 5_000,
                category_id: None,
                date: "2026-06-25".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();
        assert_eq!(balance_of(&conn, checking), 95_000);

        t.r#type = "income".into();
        update_transaction_logic(&mut conn, &t).unwrap();
        // Old -5_000 reversed (+5_000), new income +5_000 → net +5_000 from base.
        assert_eq!(balance_of(&conn, checking), 105_000);
    }

    #[test]
    fn delete_transfer_reverses_both_accounts() {
        let (mut conn, checking, savings) = setup();
        let t = insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 30_000,
                category_id: None,
                date: "2026-06-25".into(),
                note: None,
                r#type: "transfer".into(),
                transfer_account_id: Some(savings),
            },
        )
        .unwrap();
        assert_eq!(balance_of(&conn, checking), 70_000);
        assert_eq!(balance_of(&conn, savings), 80_000);

        delete_transaction_logic(&mut conn, t.id).unwrap();
        assert_eq!(balance_of(&conn, checking), 100_000);
        assert_eq!(balance_of(&conn, savings), 50_000);
    }

    #[test]
    fn update_budget_changes_amount_and_threshold() {
        let (conn, _c, _s) = setup();
        let profile_id = select_accounts(&conn).unwrap()[0].profile_id;
        let cat = insert_category(
            &conn,
            &NewCategory {
                profile_id,
                name: "Fun".into(),
                color_code: None,
                r#type: "expense".into(),
                parent_id: None,
            },
        )
        .unwrap();
        let mut budget = insert_budget(
            &conn,
            &NewBudget {
                profile_id,
                category_id: cat.id,
                period: "monthly".into(),
                amount: 10_000,
                alert_threshold: 0.9,
            },
        )
        .unwrap();

        budget.amount = 15_000;
        budget.alert_threshold = 0.75;
        let updated = update_budget_logic(&conn, &budget).unwrap();
        assert_eq!(updated.amount, 15_000);
        assert_eq!(updated.alert_threshold, 0.75);

        delete_budget_logic(&conn, budget.id).unwrap();
        assert_eq!(select_budgets(&conn).unwrap().len(), 0);
    }

    #[test]
    fn update_account_renames() {
        let (conn, checking, _s) = setup();
        let mut acc = fetch_account(&conn, checking).unwrap();
        acc.name = "Everyday".into();
        let updated = update_account_logic(&conn, &acc).unwrap();
        assert_eq!(updated.name, "Everyday");
    }

    #[test]
    fn delete_account_refused_when_referenced() {
        let (mut conn, checking, _s) = setup();
        insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: 1_000,
                category_id: None,
                date: "2026-06-25".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();

        // Referenced account cannot be deleted.
        assert!(delete_account_logic(&conn, checking).is_err());

        // An unreferenced account can.
        let profile_id = fetch_account(&conn, checking).unwrap().profile_id;
        let spare = insert_account(
            &conn,
            &NewAccount {
                profile_id,
                name: "Spare".into(),
                balance: 0,
                currency: "USD".into(),
                r#type: "cash".into(),
            },
        )
        .unwrap();
        delete_account_logic(&conn, spare.id).unwrap();
    }

    #[test]
    fn subcategory_parent_persists_and_delete_is_guarded() {
        let (conn, _c, _s) = setup();
        let profile_id = select_accounts(&conn).unwrap()[0].profile_id;

        let parent = insert_category(
            &conn,
            &NewCategory {
                profile_id,
                name: "Dining Out".into(),
                color_code: Some("#E57373".into()),
                r#type: "expense".into(),
                parent_id: None,
            },
        )
        .unwrap();

        let coffee = insert_category(
            &conn,
            &NewCategory {
                profile_id,
                name: "Coffee".into(),
                color_code: None,
                r#type: "expense".into(),
                parent_id: Some(parent.id),
            },
        )
        .unwrap();
        assert_eq!(coffee.parent_id, Some(parent.id));

        // Parent with a child can't be deleted.
        assert!(delete_category_logic(&conn, parent.id).is_err());

        // The leaf sub-category can.
        delete_category_logic(&conn, coffee.id).unwrap();
        // Now the parent (childless) can be deleted too.
        delete_category_logic(&conn, parent.id).unwrap();
    }
}
