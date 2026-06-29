import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { Category, NewCategory } from '../types'
import { createCategory, deleteCategory, listCategories, updateCategory } from '../api'

export const useCategoryStore = defineStore('category', () => {
  const data = ref<Category[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  /** Lookup map by id for fast joins (transaction -> category name/color). */
  const byId = computed<Map<number, Category>>(() => new Map(data.value.map((c) => [c.id, c])))

  /** Top-level categories (no parent). */
  const topLevel = computed<Category[]>(() => data.value.filter((c) => c.parentId == null))

  /** Direct children of a category. */
  function childrenOf(parentId: number): Category[] {
    return data.value.filter((c) => c.parentId === parentId)
  }

  /**
   * A category id plus the ids of all its (one-level) descendants. Used to roll
   * sub-category spending up to the parent — e.g. a "Dining Out" budget counts
   * transactions tagged "Coffee" or "Restaurants".
   */
  function descendantIds(categoryId: number): number[] {
    return [categoryId, ...childrenOf(categoryId).map((c) => c.id)]
  }

  /** The parent category of a sub-category, if any. */
  function parentOf(category: Category): Category | undefined {
    return category.parentId != null ? byId.value.get(category.parentId) : undefined
  }

  /** Display label including the parent, e.g. "Dining Out › Coffee". */
  function displayPath(categoryId: number | null): string {
    if (categoryId == null) return 'Uncategorized'
    const cat = byId.value.get(categoryId)
    if (!cat) return 'Unknown'
    const parent = parentOf(cat)
    return parent ? `${parent.name} › ${cat.name}` : cat.name
  }

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

  async function add(payload: NewCategory): Promise<Category> {
    const created = await createCategory(payload)
    await fetchAll()
    return created
  }

  async function update(payload: Category): Promise<Category> {
    const updated = await updateCategory(payload)
    await fetchAll()
    return updated
  }

  async function remove(id: number): Promise<void> {
    await deleteCategory(id)
    await fetchAll()
  }

  return {
    data,
    isLoading,
    error,
    byId,
    topLevel,
    childrenOf,
    descendantIds,
    parentOf,
    displayPath,
    fetchAll,
    add,
    update,
    remove,
  }
})
