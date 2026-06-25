import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Budget } from '../types'

export const useBudgetStore = defineStore('budget', () => {
  const data = ref<Budget[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      // TODO (Task 1D): replace with Tauri invoke, e.g.
      //   data.value = await invoke<Budget[]>('list_budgets')
      data.value = []
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  return { data, isLoading, error, fetchAll }
})
