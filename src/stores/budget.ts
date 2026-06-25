import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Budget } from '../types'
import { listBudgets } from '../api'

export const useBudgetStore = defineStore('budget', () => {
  const data = ref<Budget[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      data.value = await listBudgets()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  return { data, isLoading, error, fetchAll }
})
