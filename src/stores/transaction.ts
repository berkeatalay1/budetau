import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Transaction } from '../types'
import { listTransactions } from '../api'

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

  return { data, isLoading, error, fetchAll }
})
