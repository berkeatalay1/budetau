import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { NewTransaction, Transaction } from '../types'
import { createTransaction, deleteTransaction, listTransactions, updateTransaction } from '../api'
import { useAccountStore } from './account'

export const useTransactionStore = defineStore('transaction', () => {
  const data = ref<Transaction[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      data.value = await listTransactions()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  // Mutations re-fetch accounts because the backend adjusts balances
  // atomically when a transaction is created/updated/deleted.
  async function add(payload: NewTransaction): Promise<Transaction> {
    const created = await createTransaction(payload)
    await Promise.all([fetchAll(), useAccountStore().fetchAll()])
    return created
  }

  async function update(payload: Transaction): Promise<Transaction> {
    const updated = await updateTransaction(payload)
    await Promise.all([fetchAll(), useAccountStore().fetchAll()])
    return updated
  }

  async function remove(id: number): Promise<void> {
    await deleteTransaction(id)
    await Promise.all([fetchAll(), useAccountStore().fetchAll()])
  }

  return { data, isLoading, error, fetchAll, add, update, remove }
})
