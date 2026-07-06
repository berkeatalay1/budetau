import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { NewRecurringRule, RecurringRule } from '../types'
import {
  createRecurringRule,
  deleteRecurringRule,
  listRecurringRules,
  updateRecurringRule,
} from '../api'

export const useRecurringStore = defineStore('recurring', () => {
  const data = ref<RecurringRule[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      data.value = await listRecurringRules()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  async function add(payload: NewRecurringRule): Promise<RecurringRule> {
    const created = await createRecurringRule(payload)
    await fetchAll()
    return created
  }

  async function update(payload: RecurringRule): Promise<RecurringRule> {
    const updated = await updateRecurringRule(payload)
    await fetchAll()
    return updated
  }

  async function remove(id: number): Promise<void> {
    await deleteRecurringRule(id)
    await fetchAll()
  }

  return { data, isLoading, error, fetchAll, add, update, remove }
})
