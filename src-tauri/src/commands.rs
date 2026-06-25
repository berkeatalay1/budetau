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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    pub id: i64,
    pub profile_id: i64,
    pub category_id: i64,
    pub period: String,
    pub amount: i64,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBudget {
    pub profile_id: i64,
    pub category_id: i64,
    pub period: String,
    pub amount: i64,
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
    })
}

fn map_budget(row: &rusqlite::Row<'_>) -> rusqlite::Result<Budget> {
    Ok(Budget {
        id: row.get("id")?,
        profile_id: row.get("profile_id")?,
        category_id: row.get("category_id")?,
        period: row.get("period")?,
        amount: row.get("amount")?,
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
        "SELECT id, profile_id, name, color_code, type FROM categories ORDER BY id;",
    )?;
    let rows = stmt.query_map([], map_category)?;
    rows.collect()
}

fn select_budgets(conn: &Connection) -> rusqlite::Result<Vec<Budget>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, category_id, period, amount FROM budgets ORDER BY id;",
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
        "SELECT id, profile_id, name, color_code, type FROM categories WHERE id = ?1;",
        params![id],
        map_category,
    )
}

fn fetch_budget(conn: &Connection, id: i64) -> rusqlite::Result<Budget> {
    conn.query_row(
        "SELECT id, profile_id, category_id, period, amount FROM budgets WHERE id = ?1;",
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
        "INSERT INTO categories (profile_id, name, color_code, type) VALUES (?1, ?2, ?3, ?4);",
        params![new.profile_id, new.name, new.color_code, new.r#type],
    )?;
    fetch_category(conn, conn.last_insert_rowid())
}

fn insert_budget(conn: &Connection, new: &NewBudget) -> rusqlite::Result<Budget> {
    conn.execute(
        "INSERT INTO budgets (profile_id, category_id, period, amount) VALUES (?1, ?2, ?3, ?4);",
        params![new.profile_id, new.category_id, new.period, new.amount],
    )?;
    fetch_budget(conn, conn.last_insert_rowid())
}

/// Insert a transaction and atomically adjust account balance(s).
///
/// CRITICAL: the row insert and the balance update(s) happen inside one
/// rusqlite transaction, committed together, so a balance can never drift
/// out of sync with its transaction history. Balance effect by `type`:
///
/// * `expense`  — subtract `amount` from `account_id`.
/// * `income`   — add `amount` to `account_id`.
/// * `transfer` — subtract `amount` from `account_id` and add it to
///   `transfer_account_id` (which must be set).
///
/// `amount` is always stored/passed as a positive magnitude in cents; the
/// sign is derived from `type`.
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

    match new.r#type.as_str() {
        "expense" => {
            tx.execute(
                "UPDATE accounts SET balance = balance - ?1 WHERE id = ?2;",
                params![new.amount, new.account_id],
            )?;
        }
        "income" => {
            tx.execute(
                "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2;",
                params![new.amount, new.account_id],
            )?;
        }
        "transfer" => {
            let dest = new.transfer_account_id.ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    "transfer transaction requires transfer_account_id".to_string(),
                )
            })?;
            tx.execute(
                "UPDATE accounts SET balance = balance - ?1 WHERE id = ?2;",
                params![new.amount, new.account_id],
            )?;
            tx.execute(
                "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2;",
                params![new.amount, dest],
            )?;
        }
        other => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unknown transaction type: {other}"
            )));
        }
    }

    let created = fetch_transaction(&tx, id)?;
    tx.commit()?;
    Ok(created)
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
pub fn create_category(
    state: tauri::State<'_, DbState>,
    category: NewCategory,
) -> Result<Category, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    insert_category(&conn, &category).map_err(|e| e.to_string())
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
pub fn create_transaction(
    state: tauri::State<'_, DbState>,
    transaction: NewTransaction,
) -> Result<Transaction, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    insert_transaction(&mut conn, &transaction).map_err(|e| e.to_string())
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
            },
        )
        .unwrap();
        assert_eq!(budget.amount, 12_000);
        assert_eq!(select_budgets(&conn).unwrap().len(), 1);
    }
}
