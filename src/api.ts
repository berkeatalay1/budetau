// Typed wrappers around the Tauri IPC commands defined in
// `src-tauri/src/commands.rs`. Each function maps one-to-one to a
// `#[tauri::command]` and carries the right argument/return types so the
// frontend never hand-rolls `invoke` calls with stringly-typed names.

import { invoke } from '@tauri-apps/api/core'
import type {
  Account,
  Budget,
  BudgetMonth,
  Category,
  NewAccount,
  NewBudget,
  NewCategory,
  NewRecurringRule,
  NewTransaction,
  RecurringRule,
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

export const updateCategory = (category: Category): Promise<Category> =>
  invoke('update_category', { category })

export const deleteCategory = (id: number): Promise<void> => invoke('delete_category', { id })

export const updateAccount = (account: Account): Promise<Account> =>
  invoke('update_account', { account })

export const deleteAccount = (id: number): Promise<void> => invoke('delete_account', { id })

/** Rebuild every account's balance from its transaction history. */
export const recomputeBalances = (): Promise<Account[]> => invoke('recompute_balances')

export const updateBudget = (budget: Budget): Promise<Budget> => invoke('update_budget', { budget })

export const deleteBudget = (id: number): Promise<void> => invoke('delete_budget', { id })

export const updateTransaction = (transaction: Transaction): Promise<Transaction> =>
  invoke('update_transaction', { transaction })

export const deleteTransaction = (id: number): Promise<void> => invoke('delete_transaction', { id })

// --- Envelope budgeting ----------------------------------------------------

/** Fetch the Plan-view read model (per-category assigned/activity/available
 * plus Ready to Assign) for `month` ('YYYY-MM'). */
export const getBudgetMonth = (month: string): Promise<BudgetMonth> =>
  invoke('get_budget_month', { month })

/** Set a category's assigned cents for `month` to an absolute value. */
export const setAssigned = (categoryId: number, month: string, amount: number): Promise<void> =>
  invoke('set_assigned', { categoryId, month, amount })

// --- Recurring transactions ------------------------------------------------

export const listRecurringRules = (): Promise<RecurringRule[]> => invoke('list_recurring_rules')

export const createRecurringRule = (rule: NewRecurringRule): Promise<RecurringRule> =>
  invoke('create_recurring_rule', { rule })

export const updateRecurringRule = (rule: RecurringRule): Promise<RecurringRule> =>
  invoke('update_recurring_rule', { rule })

export const deleteRecurringRule = (id: number): Promise<void> =>
  invoke('delete_recurring_rule', { id })

/** Post every due recurring occurrence as of today, returning the count posted.
 * Also runs automatically on app launch; this lets the frontend trigger an
 * on-demand catch-up. */
export const postDueRecurring = (): Promise<number> => invoke('post_due_recurring_cmd')

// --- Backup & restore ----------------------------------------------------

/** Write a verified copy of the database to `path` (chosen via the dialog). */
export const exportBackup = (path: string): Promise<void> => invoke('export_backup', { path })

/** Overwrite the current database with the backup file at `path`. */
export const restoreBackup = (path: string): Promise<void> => invoke('restore_backup', { path })
