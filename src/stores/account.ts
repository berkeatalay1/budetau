import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Account, NewAccount } from '../types'
import { createAccount, deleteAccount, listAccounts, updateAccount } from '../api'

export const useAccountStore = defineStore('account', () => {
  const data = ref<Account[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      data.value = await listAccounts()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  async function add(payload: NewAccount): Promise<Account> {
    const created = await createAccount(payload)
    await fetchAll()
    return created
  }

  async function update(payload: Account): Promise<Account> {
    const updated = await updateAccount(payload)
    await fetchAll()
    return updated
  }

  /** Throws if the account still has transactions (backend guards this). */
  async function remove(id: number): Promise<void> {
    await deleteAccount(id)
    await fetchAll()
  }

  return { data, isLoading, error, fetchAll, add, update, remove }
})
