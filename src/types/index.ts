// Interim data-model interfaces for the budget planner frontend.
// NOTE: These will be reconciled with the Rust structs in Task 1D.
// Keep field names camelCase. Money fields are integers (cents).

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
