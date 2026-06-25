import { defineStore } from 'pinia'
import { ref } from 'vue'

export type QuickAddType = 'expense' | 'income' | 'transfer'

/**
 * Controls the global Quick-Add transaction modal. The app shell hosts the
 * modal and a FAB + ⌘K shortcut call `open()`; the modal component reads
 * `isOpen` and `initialType`. Decouples the shell from the modal component.
 */
export const useQuickAddStore = defineStore('quickAdd', () => {
  const isOpen = ref(false)
  const initialType = ref<QuickAddType>('expense')

  function open(type: QuickAddType = 'expense'): void {
    initialType.value = type
    isOpen.value = true
  }

  function close(): void {
    isOpen.value = false
  }

  return { isOpen, initialType, open, close }
})
