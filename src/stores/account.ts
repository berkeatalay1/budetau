import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Account } from '../types'
import { listAccounts } from '../api'

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

  return { data, isLoading, error, fetchAll }
})
