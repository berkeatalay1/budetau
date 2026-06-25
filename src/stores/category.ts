import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { Category } from '../types'
import { listCategories } from '../api'

export const useCategoryStore = defineStore('category', () => {
  const data = ref<Category[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  /** Lookup map by id for fast joins (transaction -> category name/color). */
  const byId = computed<Map<number, Category>>(() => new Map(data.value.map((c) => [c.id, c])))

  async function fetchAll(): Promise<void> {
    isLoading.value = true
    error.value = null
    try {
      data.value = await listCategories()
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      isLoading.value = false
    }
  }

  return { data, isLoading, error, byId, fetchAll }
})
