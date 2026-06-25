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
}

export interface Budget {
  id: number
  profileId: number
  categoryId: number
  period: string
  amount: number
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
}

export interface NewBudget {
  profileId: number
  categoryId: number
  period: string
  amount: number
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
