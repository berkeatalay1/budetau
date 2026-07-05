import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { BudgetMonth } from '../types'
import { getBudgetMonth, setAssigned } from '../api'

/** Current calendar month as 'YYYY-MM', in the local timezone. */
function currentMonth(): string {
  const now = new Date()
  const year = now.getFullYear()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  return `${year}-${month}`
}

export const useBudgetMonthStore = defineStore('budgetMonth', () => {
  const month = ref(currentMonth())
  const data = ref<BudgetMonth | null>(null)
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetch(): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      data.value = await getBudgetMonth(month.value)
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  async function setMonth(m: string): Promise<void> {
    month.value = m
    await fetch()
  }

  async function assign(categoryId: number, amount: number): Promise<void> {
    await setAssigned(categoryId, month.value, amount)
    await fetch()
  }

  return { month, data, isLoading, error, fetch, setMonth, assign }
})
