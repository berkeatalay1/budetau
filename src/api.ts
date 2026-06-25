// Typed wrappers around the Tauri IPC commands defined in
// `src-tauri/src/commands.rs`. Each function maps one-to-one to a
// `#[tauri::command]` and carries the right argument/return types so the
// frontend never hand-rolls `invoke` calls with stringly-typed names.

import { invoke } from '@tauri-apps/api/core'
import type {
  Account,
  Budget,
  Category,
  NewAccount,
  NewBudget,
  NewCategory,
  NewTransaction,
  Transaction,
} from './types'

// --- Database ------------------------------------------------------------

export const getDbPath = (): Promise<string> => invoke('get_db_path')

// --- Lists ---------------------------------------------------------------

export const listAccounts = (): Promise<Account[]> => invoke('list_accounts')

export const listCategories = (): Promise<Category[]> => invoke('list_categories')

export const listBudgets = (): Promise<Budget[]> => invoke('list_budgets')

export const listTransactions = (): Promise<Transaction[]> => invoke('list_transactions')

// --- Creates -------------------------------------------------------------

export const createAccount = (account: NewAccount): Promise<Account> =>
  invoke('create_account', { account })

export const createCategory = (category: NewCategory): Promise<Category> =>
  invoke('create_category', { category })

export const createBudget = (budget: NewBudget): Promise<Budget> =>
  invoke('create_budget', { budget })

export const createTransaction = (transaction: NewTransaction): Promise<Transaction> =>
  invoke('create_transaction', { transaction })

// --- Update / delete -----------------------------------------------------

export const updateTransaction = (transaction: Transaction): Promise<Transaction> =>
  invoke('update_transaction', { transaction })

export const deleteTransaction = (id: number): Promise<void> => invoke('delete_transaction', { id })
