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

export interface Budget {
  id: number
  profileId: number
  categoryId: number
  period: string
  amount: number
  /** Fraction of `amount` at which the budget is "near" its limit (0–1). */
  alertThreshold: number
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

export interface NewBudget {
  profileId: number
  categoryId: number
  period: string
  amount: number
  alertThreshold: number
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
