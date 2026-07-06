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

/// One expense category's envelope state for a single month — the per-row
/// read model behind the Plan view. See `get_budget_month` for how each
/// field is derived.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMonthCategory {
    pub category_id: i64,
    pub name: String,
    /// Signed cents assigned to this category for this month only (from
    /// `category_months`).
    pub assigned: i64,
    /// Signed net effect of this category's transactions dated in this
    /// month: income adds, expense subtracts.
    pub activity: i64,
    /// Cumulative rollover: `Σ(assigned + activity)` for this category over
    /// every month up to and including this one. May be negative
    /// (overspending carries forward).
    pub available: i64,
}

/// The envelope-budgeting read model for a single month — one row per
/// expense category plus the month's Ready to Assign figure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMonth {
    pub month: String,
    /// Unassigned money available to hand out this month (see
    /// `get_budget_month` for the exact formula).
    pub ready_to_assign: i64,
    pub categories: Vec<BudgetMonthCategory>,
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

/// A scheduled/auto-repeating transaction rule (v6 schema) — e.g. rent or a
/// biweekly paycheck. `category_id = None` on an `income` rule is exactly how
/// its posted occurrences reach Ready to Assign (see `build_budget_month`,
/// which sums all income regardless of category).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurringRule {
    pub id: i64,
    pub profile_id: i64,
    pub account_id: i64,
    pub amount: i64,
    pub category_id: Option<i64>,
    pub r#type: String,
    pub transfer_account_id: Option<i64>,
    pub note: Option<String>,
    pub cadence: String,
    /// Next occurrence to post, 'YYYY-MM-DD'. Advanced forward by
    /// `post_due_recurring` after each posted occurrence.
    pub next_due: String,
    pub active: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRecurringRule {
    pub profile_id: i64,
    pub account_id: i64,
    pub amount: i64,
    pub category_id: Option<i64>,
    pub r#type: String,
    pub transfer_account_id: Option<i64>,
    pub note: Option<String>,
    pub cadence: String,
    pub next_due: String,
    /// Defaults to true when omitted by the frontend.
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
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

fn map_recurring_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecurringRule> {
    Ok(RecurringRule {
        id: row.get("id")?,
        profile_id: row.get("profile_id")?,
        account_id: row.get("account_id")?,
        amount: row.get("amount")?,
        category_id: row.get("category_id")?,
        r#type: row.get("type")?,
        transfer_account_id: row.get("transfer_account_id")?,
        note: row.get("note")?,
        cadence: row.get("cadence")?,
        next_due: row.get("next_due")?,
        active: row.get("active")?,
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

/// Insert a new account. `opening_balance` is set equal to `balance` at
/// creation time — there are no transactions yet, so the starting balance
/// and the opening balance are the same value. `opening_balance` is an
/// internal DB-layer column only (see `db::migrate_v4`); it is never exposed
/// on the `Account` struct or the IPC surface.
fn insert_account(conn: &Connection, new: &NewAccount) -> rusqlite::Result<Account> {
    conn.execute(
        "INSERT INTO accounts (profile_id, name, balance, currency, type, opening_balance) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?3);",
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
    // `amount` is a positive magnitude by invariant; abs() defensively so a
    // stray negative can never invert a balance update.
    let amt = amount.abs() * dir;
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

    // Store a positive magnitude; the sign is always derived from `type`.
    let amount = new.amount.abs();
    tx.execute(
        "INSERT INTO transactions \
         (account_id, amount, category_id, date, note, type, transfer_account_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
        params![
            new.account_id,
            amount,
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
        amount,
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

    // Store a positive magnitude; the sign is always derived from `type`.
    let amount = t.amount.abs();
    tx.execute(
        "UPDATE transactions SET \
         account_id = ?1, amount = ?2, category_id = ?3, date = ?4, note = ?5, \
         type = ?6, transfer_account_id = ?7 WHERE id = ?8;",
        params![
            t.account_id,
            amount,
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
        amount,
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

/// Rebuild every account's `balance` from `opening_balance` plus the net
/// effect of its transaction history, as a safety net against the
/// incremental-balance drift described by invariant ② (balances are mutated
/// incrementally on every insert/update/delete, never recomputed from
/// scratch — a bad write can drift a balance out of sync).
///
/// The net effect per account mirrors `adjust_balances` exactly: `income`
/// adds, `expense` subtracts, `transfer` subtracts from the source
/// (`account_id`) and adds to the destination (`transfer_account_id`). Using
/// `opening_balance` (rather than recomputing from transactions alone) is
/// what preserves each account's starting balance — see `db::migrate_v4`.
fn recompute_balances_logic(conn: &Connection) -> rusqlite::Result<Vec<Account>> {
    conn.execute_batch(
        "
        UPDATE accounts SET balance = opening_balance + (
            COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'income' AND account_id = accounts.id), 0)
            - COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'expense' AND account_id = accounts.id), 0)
            - COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'transfer' AND account_id = accounts.id), 0)
            + COALESCE((SELECT SUM(amount) FROM transactions WHERE type = 'transfer' AND transfer_account_id = accounts.id), 0)
        );
        ",
    )?;
    select_accounts(conn)
}

// ---------------------------------------------------------------------------
// Envelope budgeting engine — `category_months` (v5) read/write.
//
// Money is signed cents here, unlike transaction amounts: an envelope's
// `assigned`/`activity`/`available` may legitimately go negative (overspend,
// or money moved back out of a category). Month comparison is done lexically
// on the 'YYYY-MM' string, which sorts correctly for that format.
// ---------------------------------------------------------------------------

/// Build the envelope-budgeting read model for `month` ('YYYY-MM'): one row
/// per expense category (parents and subs alike, ordered by id) plus the
/// month's Ready to Assign figure.
///
/// Per category `C`:
/// * `assigned` — that category's `category_months.assigned` for exactly
///   `:month` (0 if no row exists yet).
/// * `activity` — signed sum of `C`'s transactions dated in `:month`: income
///   adds, expense subtracts (transfers never carry a category).
/// * `available` — cumulative rollover: `Σ(assigned + activity)` across
///   every month `<= :month`, i.e. all of `C`'s assigned cents plus all of
///   `C`'s transaction activity up to and including `:month`.
///
/// Ready to Assign (a single figure, not per-category):
/// `inflows − Σ assigned(all categories, months <= :month) − uncategorized
/// outflow(months <= :month)`, where `inflows` is income transactions dated
/// `<= :month` plus every account's `opening_balance` (all accounts are
/// on-budget in v1, so opening balances are starting money to assign) and
/// `uncategorized outflow` is `expense` transactions with no `category_id`
/// dated `<= :month` (money that left with no envelope reduces what's left
/// to assign). Transfers between on-budget accounts net to zero and are
/// ignored throughout.
pub fn build_budget_month(conn: &Connection, month: &str) -> rusqlite::Result<BudgetMonth> {
    let mut stmt = conn.prepare(
        "SELECT
            c.id,
            c.name,
            COALESCE((
                SELECT cm.assigned FROM category_months cm
                WHERE cm.category_id = c.id AND cm.month = :month
            ), 0) AS assigned,
            COALESCE((
                SELECT SUM(CASE WHEN t.type = 'income' THEN t.amount ELSE -t.amount END)
                FROM transactions t
                WHERE t.category_id = c.id AND substr(t.date, 1, 7) = :month
            ), 0) AS activity,
            COALESCE((
                SELECT SUM(cm.assigned) FROM category_months cm
                WHERE cm.category_id = c.id AND cm.month <= :month
            ), 0)
            +
            COALESCE((
                SELECT SUM(CASE WHEN t.type = 'income' THEN t.amount ELSE -t.amount END)
                FROM transactions t
                WHERE t.category_id = c.id AND substr(t.date, 1, 7) <= :month
            ), 0) AS available
         FROM categories c
         WHERE c.type = 'expense'
         ORDER BY c.id;",
    )?;
    let categories = stmt
        .query_map(
            rusqlite::named_params! { ":month": month },
            |row| {
                Ok(BudgetMonthCategory {
                    category_id: row.get(0)?,
                    name: row.get(1)?,
                    assigned: row.get(2)?,
                    activity: row.get(3)?,
                    available: row.get(4)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let ready_to_assign: i64 = conn.query_row(
        "SELECT
            COALESCE((
                SELECT SUM(amount) FROM transactions
                WHERE type = 'income' AND substr(date, 1, 7) <= :month
            ), 0)
            +
            COALESCE((SELECT SUM(opening_balance) FROM accounts), 0)
            -
            COALESCE((
                SELECT SUM(assigned) FROM category_months WHERE month <= :month
            ), 0)
            -
            COALESCE((
                SELECT SUM(amount) FROM transactions
                WHERE type = 'expense' AND category_id IS NULL AND substr(date, 1, 7) <= :month
            ), 0);",
        rusqlite::named_params! { ":month": month },
        |row| row.get(0),
    )?;

    Ok(BudgetMonth {
        month: month.to_string(),
        ready_to_assign,
        categories,
    })
}

/// Set a category's `assigned` cents for `month` to an **absolute** value
/// (not a delta): upserts `category_months`, overwriting any existing row
/// for `(category_id, month)`.
///
/// Absolute semantics are intentional — a later "move money" feature (moving
/// funds between two envelopes, or from Ready to Assign) computes each
/// side's new total from its current `available`/`assigned` and calls this
/// with the resulting absolute figure, rather than needing a separate
/// delta-based API.
pub fn set_assigned_logic(
    conn: &Connection,
    category_id: i64,
    month: &str,
    amount: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO category_months (profile_id, category_id, month, assigned)
         VALUES ((SELECT profile_id FROM categories WHERE id = ?1), ?1, ?2, ?3)
         ON CONFLICT(category_id, month) DO UPDATE SET assigned = excluded.assigned;",
        params![category_id, month, amount],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Recurring transactions — CRUD plus the launch-time scheduler.
//
// A `recurring_rules` row (v6 schema) describes a scheduled/auto-repeating
// transaction (rent, salary, ...). `post_due_recurring` is the scheduler: it
// posts every occurrence whose `next_due` has arrived (catching up on any
// missed/back-dated occurrences too), reusing `insert_transaction` so balance
// math and the Ready to Assign flow (income with `category_id = None`) work
// exactly as they do for a manually entered transaction.
// ---------------------------------------------------------------------------

fn select_recurring_rules(conn: &Connection) -> rusqlite::Result<Vec<RecurringRule>> {
    let mut stmt = conn.prepare(
        "SELECT id, profile_id, account_id, amount, category_id, type, \
         transfer_account_id, note, cadence, next_due, active \
         FROM recurring_rules ORDER BY id;",
    )?;
    let rows = stmt.query_map([], map_recurring_rule)?;
    rows.collect()
}

fn fetch_recurring_rule(conn: &Connection, id: i64) -> rusqlite::Result<RecurringRule> {
    conn.query_row(
        "SELECT id, profile_id, account_id, amount, category_id, type, \
         transfer_account_id, note, cadence, next_due, active \
         FROM recurring_rules WHERE id = ?1;",
        params![id],
        map_recurring_rule,
    )
}

fn insert_recurring_rule(
    conn: &Connection,
    new: &NewRecurringRule,
) -> rusqlite::Result<RecurringRule> {
    conn.execute(
        "INSERT INTO recurring_rules \
         (profile_id, account_id, amount, category_id, type, transfer_account_id, note, \
          cadence, next_due, active) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
        params![
            new.profile_id,
            new.account_id,
            new.amount.abs(),
            new.category_id,
            new.r#type,
            new.transfer_account_id,
            new.note,
            new.cadence,
            new.next_due,
            new.active,
        ],
    )?;
    fetch_recurring_rule(conn, conn.last_insert_rowid())
}

fn update_recurring_rule_logic(
    conn: &Connection,
    r: &RecurringRule,
) -> rusqlite::Result<RecurringRule> {
    conn.execute(
        "UPDATE recurring_rules SET \
         account_id = ?1, amount = ?2, category_id = ?3, type = ?4, transfer_account_id = ?5, \
         note = ?6, cadence = ?7, next_due = ?8, active = ?9 WHERE id = ?10;",
        params![
            r.account_id,
            r.amount.abs(),
            r.category_id,
            r.r#type,
            r.transfer_account_id,
            r.note,
            r.cadence,
            r.next_due,
            r.active,
            r.id,
        ],
    )?;
    fetch_recurring_rule(conn, r.id)
}

fn delete_recurring_rule_logic(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM recurring_rules WHERE id = ?1;", params![id])?;
    Ok(())
}

/// Advance `date` forward by one `cadence` period using SQLite's `date()`
/// modifiers.
///
/// NOTE: `'+1 month'` has SQLite's well-known month-end overflow quirk — e.g.
/// Jan 31 + 1 month lands on Mar 03, not Feb 28/29, because SQLite clamps the
/// day *after* rolling the month forward. Acceptable for v1; a future version
/// could special-case month-end dates if this proves confusing in practice.
fn advance_date(conn: &Connection, date: &str, cadence: &str) -> rusqlite::Result<String> {
    let modifier = match cadence {
        "weekly" => "+7 days",
        "biweekly" => "+14 days",
        "monthly" => "+1 month",
        "yearly" => "+1 year",
        other => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unknown cadence: {other}"
            )))
        }
    };
    conn.query_row(
        "SELECT date(?1, ?2);",
        params![date, modifier],
        |row| row.get(0),
    )
}

/// Post every due (or missed/back-dated) occurrence of every active
/// recurring rule as of `today` ('YYYY-MM-DD'), returning the total number of
/// transactions posted.
///
/// For each active rule, while `next_due <= today`: post a transaction dated
/// `next_due` via `insert_transaction` (so balances adjust exactly as a
/// manual entry would, and income with `category_id = None` reaches Ready to
/// Assign), then advance `next_due` by the rule's cadence and persist it.
/// This both catches up multiple missed occurrences in one call and makes
/// repeated calls on the same `today` idempotent — once `next_due` passes
/// `today`, the loop for that rule stops posting.
///
/// The per-rule catch-up loop is capped at 1000 iterations so a corrupted or
/// absurdly old `next_due` can't spin forever.
pub fn post_due_recurring(conn: &mut Connection, today: &str) -> rusqlite::Result<usize> {
    const MAX_CATCH_UP: usize = 1000;

    // Collect the rule rows first, into an owned Vec, so the prepared
    // statement's borrow of `conn` ends before we mutate via
    // `insert_transaction` (which needs `&mut Connection`).
    let rules: Vec<RecurringRule> = {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, account_id, amount, category_id, type, \
             transfer_account_id, note, cadence, next_due, active \
             FROM recurring_rules WHERE active = 1 ORDER BY id;",
        )?;
        let rows = stmt.query_map([], map_recurring_rule)?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let mut posted = 0usize;
    for rule in rules {
        let mut next_due = rule.next_due.clone();
        let mut iterations = 0usize;
        while next_due.as_str() <= today && iterations < MAX_CATCH_UP {
            insert_transaction(
                conn,
                &NewTransaction {
                    account_id: rule.account_id,
                    amount: rule.amount,
                    category_id: rule.category_id,
                    date: next_due.clone(),
                    note: rule.note.clone(),
                    r#type: rule.r#type.clone(),
                    transfer_account_id: rule.transfer_account_id,
                },
            )?;
            posted += 1;
            next_due = advance_date(conn, &next_due, &rule.cadence)?;
            iterations += 1;
        }
        if next_due != rule.next_due {
            conn.execute(
                "UPDATE recurring_rules SET next_due = ?1 WHERE id = ?2;",
                params![next_due, rule.id],
            )?;
        }
    }

    Ok(posted)
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

/// Rebuild every account's `balance` from scratch against its transaction
/// history — a maintenance action for the Settings "Recalculate balances"
/// button, guarding against the incremental-update drift in invariant ②.
#[tauri::command]
pub fn recompute_balances(state: tauri::State<'_, DbState>) -> Result<Vec<Account>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    recompute_balances_logic(&conn).map_err(|e| e.to_string())
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
// Envelope budgeting — Plan-view read model and the one write path (v1: set
// an absolute assigned amount). See the plain-fn doc comments above for the
// exact formulas.
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_budget_month(
    state: tauri::State<'_, DbState>,
    month: String,
) -> Result<BudgetMonth, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    build_budget_month(&conn, &month).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_assigned(
    state: tauri::State<'_, DbState>,
    category_id: i64,
    month: String,
    amount: i64,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    set_assigned_logic(&conn, category_id, &month, amount).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Recurring transactions — CRUD + the scheduler command.
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_recurring_rules(state: tauri::State<'_, DbState>) -> Result<Vec<RecurringRule>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    select_recurring_rules(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_recurring_rule(
    state: tauri::State<'_, DbState>,
    rule: NewRecurringRule,
) -> Result<RecurringRule, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    insert_recurring_rule(&conn, &rule).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_recurring_rule(
    state: tauri::State<'_, DbState>,
    rule: RecurringRule,
) -> Result<RecurringRule, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_recurring_rule_logic(&conn, &rule).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_recurring_rule(state: tauri::State<'_, DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    delete_recurring_rule_logic(&conn, id).map_err(|e| e.to_string())
}

/// Post every due recurring occurrence as of today (local time), returning
/// the count posted. Also invoked once automatically on app launch (see
/// `lib.rs` `setup()`, which calls the plain `post_due_recurring` fn
/// directly); exposed as a command too so the frontend can trigger an
/// on-demand catch-up (e.g. a manual refresh button in Wave 1).
#[tauri::command]
pub fn post_due_recurring_cmd(state: tauri::State<'_, DbState>) -> Result<usize, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    let today: String = conn
        .query_row("SELECT date('now','localtime');", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    post_due_recurring(&mut conn, &today).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Backup & restore — copy the live SQLite file to/from a user-chosen path.
// The frontend picks the path via the dialog plugin; these commands do the
// file work and verify integrity. Money/data invariants are untouched.
// ---------------------------------------------------------------------------

/// Export a verified copy of the live database to `path`.
///
/// Holds the DB lock for the duration so no write is in flight; with WAL
/// disabled the on-disk file is then a consistent snapshot that is safe to
/// copy. The copy is integrity-checked before the command returns.
#[tauri::command]
pub fn export_backup(
    app: AppHandle,
    state: tauri::State<'_, DbState>,
    path: String,
) -> Result<(), String> {
    let db_path = db::resolve_db_path(&app)?;
    // Hold the lock across the copy to block concurrent writes.
    let _guard = state.0.lock().map_err(|e| e.to_string())?;
    db::export_to(&db_path, std::path::Path::new(&path))
}

/// Restore the database from the backup file at `path`, overwriting the
/// current data.
///
/// Steps, all under the DB lock: (1) verify the backup passes
/// `integrity_check`; (2) take an automatic timestamped backup of the current
/// database next to it (the UI promises this), then prune old safety copies so
/// they don't accumulate; (3) drop the live connection, copy the backup over
/// the live file, then reopen and re-run `init` + `seed_defaults` so an
/// older-schema backup is migrated forward and the "at least one account"
/// invariant still holds.
#[tauri::command]
pub fn restore_backup(
    app: AppHandle,
    state: tauri::State<'_, DbState>,
    path: String,
) -> Result<(), String> {
    let db_path = db::resolve_db_path(&app)?;
    let backup_path = std::path::Path::new(&path);

    // 1. Refuse to restore from a file that is not an intact database.
    db::check_integrity(backup_path)?;

    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    // 2. Safety copy of the current data before we overwrite it.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let auto_backup = db_path.with_file_name(format!("{}{stamp}.db", db::PRE_RESTORE_PREFIX));
    std::fs::copy(&db_path, &auto_backup)
        .map_err(|e| format!("failed to back up current database before restore: {e}"))?;
    // Keep only the most recent safety copies (best-effort; never abort a restore).
    if let Some(dir) = db_path.parent() {
        let _ = db::prune_pre_restore_backups(dir, 5);
    }

    // 3. Drop the live connection (placeholder in-memory db) so no handle is
    //    open on the file, overwrite it, then reopen the restored database.
    *guard = Connection::open_in_memory()
        .map_err(|e| format!("failed to release the database during restore: {e}"))?;
    std::fs::copy(backup_path, &db_path)
        .map_err(|e| format!("failed to overwrite the database with the backup: {e}"))?;

    let conn =
        Connection::open(&db_path).map_err(|e| format!("failed to reopen restored database: {e}"))?;
    db::init(&conn).map_err(|e| format!("failed to initialise restored database: {e}"))?;
    db::seed_defaults(&conn).map_err(|e| format!("failed to seed restored database: {e}"))?;
    *guard = conn;
    Ok(())
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

    #[test]
    fn negative_amount_is_normalized_to_positive_magnitude() {
        let (mut conn, checking, _s) = setup();

        // A buggy client sends a signed (negative) expense amount.
        let t = insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: checking,
                amount: -2_500,
                category_id: None,
                date: "2026-06-25".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();

        // Stored as a positive magnitude, and the balance went DOWN (expense),
        // not up — i.e. the sign came from `type`, not the amount.
        assert_eq!(t.amount, 2_500);
        assert_eq!(balance_of(&conn, checking), 100_000 - 2_500);

        // Same protection on update.
        let mut edited = t.clone();
        edited.amount = -4_000;
        let u = update_transaction_logic(&mut conn, &edited).unwrap();
        assert_eq!(u.amount, 4_000);
        assert_eq!(balance_of(&conn, checking), 100_000 - 4_000);
    }

    #[test]
    fn recompute_fixes_drift_from_a_corrupted_balance() {
        let (mut conn, checking, savings) = setup();

        insert_transaction(
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

        // Directly corrupt the balance with a raw UPDATE, bypassing
        // adjust_balances entirely — simulating the drift invariant ② warns
        // about.
        conn.execute(
            "UPDATE accounts SET balance = 999_999 WHERE id = ?1;",
            params![checking],
        )
        .unwrap();
        assert_eq!(balance_of(&conn, checking), 999_999);

        let recomputed = recompute_balances_logic(&conn).unwrap();

        assert_eq!(balance_of(&conn, checking), 97_500, "recompute should undo the corruption");
        assert_eq!(balance_of(&conn, savings), 50_000, "untouched account should be unaffected");

        let checking_out = recomputed.iter().find(|a| a.id == checking).unwrap();
        assert_eq!(checking_out.balance, 97_500, "returned accounts should reflect the fix");
    }

    #[test]
    fn recompute_preserves_opening_balance_not_just_transaction_sum() {
        let (mut conn, checking, _savings) = setup();
        // setup() creates `checking` with balance 100_000 (its opening
        // balance) and no transactions yet.

        insert_transaction(
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

        recompute_balances_logic(&conn).unwrap();

        // Must stay 97_500 (opening_balance 100_000 - 2_500), NOT 2_500
        // (transaction sum alone) and NOT -2_500.
        assert_eq!(balance_of(&conn, checking), 97_500);
    }

    #[test]
    fn recompute_accounts_for_both_sides_of_a_transfer() {
        let (mut conn, checking, savings) = setup();

        insert_transaction(
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

        // Corrupt both sides.
        conn.execute("UPDATE accounts SET balance = 0;", []).unwrap();

        recompute_balances_logic(&conn).unwrap();

        assert_eq!(balance_of(&conn, checking), 70_000, "source should be debited");
        assert_eq!(balance_of(&conn, savings), 80_000, "destination should be credited");
    }

    // -----------------------------------------------------------------
    // Envelope budgeting engine (`get_budget_month` / `set_assigned`).
    // -----------------------------------------------------------------

    /// A minimal DB for budgeting tests: one profile, one zero-balance
    /// account (so `opening_balance` doesn't skew `ready_to_assign` maths),
    /// and one expense category. Returns (conn, account_id, category_id).
    fn setup_budgeting() -> (Connection, i64, i64) {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO app_profiles (profile_name, created_at) VALUES ('Test', datetime('now'));",
            [],
        )
        .unwrap();
        let profile_id = conn.last_insert_rowid();

        let account = insert_account(
            &conn,
            &NewAccount {
                profile_id,
                name: "Checking".into(),
                balance: 0,
                currency: "USD".into(),
                r#type: "checking".into(),
            },
        )
        .unwrap();

        let category = insert_category(
            &conn,
            &NewCategory {
                profile_id,
                name: "Groceries".into(),
                color_code: None,
                r#type: "expense".into(),
                parent_id: None,
            },
        )
        .unwrap();

        (conn, account.id, category.id)
    }

    #[test]
    fn assign_then_spend_yields_expected_available() {
        let (mut conn, account, category) = setup_budgeting();

        set_assigned_logic(&conn, category, "2026-07", 20_000).unwrap();
        insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: account,
                amount: 5_000,
                category_id: Some(category),
                date: "2026-07-10".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();

        let month = build_budget_month(&conn, "2026-07").unwrap();
        let row = month
            .categories
            .iter()
            .find(|c| c.category_id == category)
            .unwrap();
        assert_eq!(row.assigned, 20_000);
        assert_eq!(row.activity, -5_000);
        assert_eq!(row.available, 15_000);
    }

    #[test]
    fn rollover_carries_leftover_into_next_month() {
        let (mut conn, account, category) = setup_budgeting();

        // M1: assign 10000, spend 4000.
        set_assigned_logic(&conn, category, "2026-06", 10_000).unwrap();
        insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: account,
                amount: 4_000,
                category_id: Some(category),
                date: "2026-06-15".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();

        // M2: assign another 10000.
        set_assigned_logic(&conn, category, "2026-07", 10_000).unwrap();

        let month = build_budget_month(&conn, "2026-07").unwrap();
        let row = month
            .categories
            .iter()
            .find(|c| c.category_id == category)
            .unwrap();
        // 6000 carried over (10000 - 4000) + 10000 assigned in M2.
        assert_eq!(row.available, 16_000);
    }

    #[test]
    fn ready_to_assign_responds_to_inflow_and_assignment() {
        let (conn, _account, category) = setup_budgeting();

        let before = build_budget_month(&conn, "2026-07").unwrap();
        assert_eq!(before.ready_to_assign, 0);

        // An income transaction raises ready_to_assign by its amount.
        // Category must be None (income is not tagged to an envelope here).
        conn.execute(
            "INSERT INTO transactions (account_id, amount, category_id, date, type) \
             VALUES ((SELECT id FROM accounts LIMIT 1), 50000, NULL, '2026-07-05', 'income');",
            [],
        )
        .unwrap();

        let after_income = build_budget_month(&conn, "2026-07").unwrap();
        assert_eq!(after_income.ready_to_assign, 50_000);

        // Assigning to a category lowers ready_to_assign by the same amount.
        set_assigned_logic(&conn, category, "2026-07", 15_000).unwrap();
        let after_assign = build_budget_month(&conn, "2026-07").unwrap();
        assert_eq!(after_assign.ready_to_assign, 50_000 - 15_000);
    }

    #[test]
    fn overspend_makes_available_negative() {
        let (mut conn, account, category) = setup_budgeting();

        set_assigned_logic(&conn, category, "2026-07", 1_000).unwrap();
        insert_transaction(
            &mut conn,
            &NewTransaction {
                account_id: account,
                amount: 3_000,
                category_id: Some(category),
                date: "2026-07-10".into(),
                note: None,
                r#type: "expense".into(),
                transfer_account_id: None,
            },
        )
        .unwrap();

        let month = build_budget_month(&conn, "2026-07").unwrap();
        let row = month
            .categories
            .iter()
            .find(|c| c.category_id == category)
            .unwrap();
        assert_eq!(row.available, -2_000);
    }

    // -----------------------------------------------------------------
    // Recurring transactions (`advance_date` / `post_due_recurring`).
    // -----------------------------------------------------------------

    #[test]
    fn advance_date_matches_each_cadence() {
        let (conn, _account, _category) = setup_budgeting();

        assert_eq!(advance_date(&conn, "2026-07-01", "weekly").unwrap(), "2026-07-08");
        assert_eq!(advance_date(&conn, "2026-07-01", "biweekly").unwrap(), "2026-07-15");
        assert_eq!(advance_date(&conn, "2026-07-01", "monthly").unwrap(), "2026-08-01");
        assert_eq!(advance_date(&conn, "2026-07-01", "yearly").unwrap(), "2027-07-01");
    }

    /// Insert a recurring rule directly (there is no unit under test for the
    /// insert path itself beyond plain SQL, so tests build rows straight from
    /// `NewRecurringRule` via `insert_recurring_rule`).
    fn make_rule(conn: &Connection, new: NewRecurringRule) -> RecurringRule {
        insert_recurring_rule(conn, &new).unwrap()
    }

    #[test]
    fn post_due_recurring_catches_up_missed_occurrences_and_advances_next_due() {
        let (mut conn, account, category) = setup_budgeting();
        let profile_id = fetch_account(&conn, account).unwrap().profile_id;

        // Biweekly expense rule, next_due several periods in the past
        // relative to "today" = 2026-07-06.
        let rule = make_rule(
            &conn,
            NewRecurringRule {
                profile_id,
                account_id: account,
                amount: 1_000,
                category_id: Some(category),
                r#type: "expense".into(),
                transfer_account_id: None,
                note: Some("Rent installment".into()),
                cadence: "biweekly".into(),
                next_due: "2026-06-08".into(), // 4 periods before 2026-07-06
                active: true,
            },
        );

        let posted = post_due_recurring(&mut conn, "2026-07-06").unwrap();
        // Occurrences due on/before 2026-07-06: 06-08, 06-22, 07-06 = 3.
        assert_eq!(posted, 3);

        let updated = fetch_recurring_rule(&conn, rule.id).unwrap();
        assert_eq!(updated.next_due, "2026-07-20", "next_due should be the first date > today");

        // Balance reflects all 3 posted occurrences (each an expense of 1000).
        assert_eq!(balance_of(&conn, account), 0 - 3 * 1_000);

        let txn_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(txn_count, 3);
    }

    #[test]
    fn post_due_recurring_biweekly_income_reaches_ready_to_assign() {
        let (mut conn, account, _category) = setup_budgeting();
        let profile_id = fetch_account(&conn, account).unwrap().profile_id;

        // Biweekly paycheck: category_id = None so it flows to Ready to Assign.
        make_rule(
            &conn,
            NewRecurringRule {
                profile_id,
                account_id: account,
                amount: 200_000,
                category_id: None,
                r#type: "income".into(),
                transfer_account_id: None,
                note: Some("Paycheck".into()),
                cadence: "biweekly".into(),
                next_due: "2026-07-06".into(),
                active: true,
            },
        );

        let before = build_budget_month(&conn, "2026-07").unwrap();
        let posted = post_due_recurring(&mut conn, "2026-07-06").unwrap();
        assert_eq!(posted, 1);

        let after = build_budget_month(&conn, "2026-07").unwrap();
        assert_eq!(
            after.ready_to_assign - before.ready_to_assign,
            200_000,
            "biweekly income should increase Ready to Assign by the posted amount"
        );
    }

    #[test]
    fn post_due_recurring_is_idempotent_for_the_same_today() {
        let (mut conn, account, category) = setup_budgeting();
        let profile_id = fetch_account(&conn, account).unwrap().profile_id;

        make_rule(
            &conn,
            NewRecurringRule {
                profile_id,
                account_id: account,
                amount: 5_000,
                category_id: Some(category),
                r#type: "expense".into(),
                transfer_account_id: None,
                note: None,
                cadence: "monthly".into(),
                next_due: "2026-07-06".into(),
                active: true,
            },
        );

        let first = post_due_recurring(&mut conn, "2026-07-06").unwrap();
        assert_eq!(first, 1);

        let second = post_due_recurring(&mut conn, "2026-07-06").unwrap();
        assert_eq!(second, 0, "calling again with the same today must post nothing new");
    }

    #[test]
    fn post_due_recurring_skips_inactive_rules() {
        let (mut conn, account, category) = setup_budgeting();
        let profile_id = fetch_account(&conn, account).unwrap().profile_id;

        make_rule(
            &conn,
            NewRecurringRule {
                profile_id,
                account_id: account,
                amount: 5_000,
                category_id: Some(category),
                r#type: "expense".into(),
                transfer_account_id: None,
                note: None,
                cadence: "weekly".into(),
                next_due: "2026-01-01".into(), // long overdue
                active: false,
            },
        );

        let posted = post_due_recurring(&mut conn, "2026-07-06").unwrap();
        assert_eq!(posted, 0, "an inactive rule must post nothing");

        let txn_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(txn_count, 0);
    }

    #[test]
    fn recurring_rule_crud_roundtrip() {
        let (conn, account, category) = setup_budgeting();
        let profile_id = fetch_account(&conn, account).unwrap().profile_id;

        let created = make_rule(
            &conn,
            NewRecurringRule {
                profile_id,
                account_id: account,
                amount: 3_000,
                category_id: Some(category),
                r#type: "expense".into(),
                transfer_account_id: None,
                note: Some("Gym".into()),
                cadence: "monthly".into(),
                next_due: "2026-08-01".into(),
                active: true,
            },
        );
        assert_eq!(select_recurring_rules(&conn).unwrap().len(), 1);

        let mut updated = created.clone();
        updated.amount = 3_500;
        updated.active = false;
        let saved = update_recurring_rule_logic(&conn, &updated).unwrap();
        assert_eq!(saved.amount, 3_500);
        assert!(!saved.active);

        delete_recurring_rule_logic(&conn, created.id).unwrap();
        assert_eq!(select_recurring_rules(&conn).unwrap().len(), 0);
    }
}
