import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { Category } from '../../types'

// The store imports the Tauri IPC wrappers; stub them so nothing hits `invoke`.
vi.mock('../../api', () => ({
  listCategories: vi.fn(),
  createCategory: vi.fn(),
  updateCategory: vi.fn(),
  deleteCategory: vi.fn(),
}))

import { useCategoryStore } from '../category'

/** Minimal category factory. */
function cat(id: number, name: string, parentId: number | null = null): Category {
  return { id, profileId: 1, name, colorCode: null, type: 'expense', parentId }
}

describe('category store hierarchy helpers', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  // Dining Out (1) → Coffee (2), Restaurants (3); Groceries (4) top-level.
  function seeded() {
    const store = useCategoryStore()
    store.data = [
      cat(1, 'Dining Out'),
      cat(2, 'Coffee', 1),
      cat(3, 'Restaurants', 1),
      cat(4, 'Groceries'),
    ]
    return store
  }

  it('topLevel returns only parentless categories', () => {
    const store = seeded()
    expect(store.topLevel.map((c) => c.name)).toEqual(['Dining Out', 'Groceries'])
  })

  it('childrenOf returns direct children', () => {
    const store = seeded()
    expect(store.childrenOf(1).map((c) => c.id)).toEqual([2, 3])
    expect(store.childrenOf(4)).toEqual([])
  })

  it('descendantIds includes the category itself plus its children', () => {
    const store = seeded()
    expect(store.descendantIds(1)).toEqual([1, 2, 3])
    // A leaf rolls up to just itself.
    expect(store.descendantIds(4)).toEqual([4])
  })

  it('parentOf resolves the parent, or undefined for a top-level category', () => {
    const store = seeded()
    expect(store.parentOf(cat(2, 'Coffee', 1))?.name).toBe('Dining Out')
    expect(store.parentOf(cat(1, 'Dining Out'))).toBeUndefined()
  })

  it('displayPath renders "Parent › Child", plain name, or fallbacks', () => {
    const store = seeded()
    expect(store.displayPath(2)).toBe('Dining Out › Coffee')
    expect(store.displayPath(1)).toBe('Dining Out')
    expect(store.displayPath(null)).toBe('Uncategorized')
    expect(store.displayPath(999)).toBe('Unknown')
  })

  it('byId maps ids to categories', () => {
    const store = seeded()
    expect(store.byId.get(3)?.name).toBe('Restaurants')
    expect(store.byId.has(999)).toBe(false)
  })
})
