// Data-model interfaces for the budget planner frontend.
// These mirror the serde structs in `src-tauri/src/commands.rs`, which emit
// camelCase via `#[serde(rename_all = "camelCase")]`.
// Money fields are integers (cents); never floats.

export interface Account {
  id: number
  profileId: number
  name: string
  balance: number
  currency: string
  type: string
}

export interface Category {
  id: number
  profileId: number
  name: string
  colorCode: string | null
  type: string
  /** Parent category id, or null for a top-level category. */
  parentId: number | null
}

// Envelope budgeting (v5 schema) — the Plan-view read model.

export interface BudgetMonthCategory {
  categoryId: number
  name: string
  /** Signed cents assigned to this category for this month only. */
  assigned: number
  /** Signed net effect of this category's transactions this month. */
  activity: number
  /** Cumulative rollover through this month; may be negative. */
  available: number
}

export interface BudgetMonth {
  month: string
  /** Unassigned money available to hand out this month. */
  readyToAssign: number
  categories: BudgetMonthCategory[]
}

export interface Transaction {
  id: number
  accountId: number
  amount: number
  categoryId: number | null
  date: string
  note: string | null
  type: string
  transferAccountId: number | null
}

// Create payloads — `id` is assigned by SQLite and omitted on insert.
// Mirror the `New*` structs in `commands.rs`.

export interface NewAccount {
  profileId: number
  name: string
  balance?: number
  currency: string
  type: string
}

export interface NewCategory {
  profileId: number
  name: string
  colorCode: string | null
  type: string
  parentId?: number | null
}

export interface NewTransaction {
  accountId: number
  amount: number
  categoryId: number | null
  date: string
  note: string | null
  type: string
  transferAccountId: number | null
}

// Recurring transactions (v6 schema) — scheduled/auto-repeating transactions
// (rent, salary, ...) posted automatically by the backend scheduler on
// launch. A `categoryId` of null on an income rule is how its occurrences
// reach Ready to Assign, same as a manually entered income transaction.

export interface RecurringRule {
  id: number
  profileId: number
  accountId: number
  amount: number
  categoryId: number | null
  type: string
  transferAccountId: number | null
  note: string | null
  cadence: string
  /** Next occurrence to post, 'YYYY-MM-DD'. */
  nextDue: string
  active: boolean
}

export interface NewRecurringRule {
  profileId: number
  accountId: number
  amount: number
  categoryId: number | null
  type: string
  transferAccountId: number | null
  note: string | null
  cadence: string
  nextDue: string
  /** Defaults to true when omitted. */
  active?: boolean
}
