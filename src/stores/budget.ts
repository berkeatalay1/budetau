import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Budget, NewBudget } from '../types'
import { createBudget, listBudgets } from '../api'

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

  async function add(payload: NewBudget): Promise<Budget> {
    const created = await createBudget(payload)
    await fetchAll()
    return created
  }

  return { data, isLoading, error, fetchAll, add }
})
