import { defineStore } from 'pinia'
import { ref } from 'vue'

/**
 * Global toast store — a single transient status message rendered once by
 * `ToastHost.vue` (mounted in `App.vue`). Any view or component can call
 * `show()` instead of hand-rolling its own toast state and markup.
 *
 * `error: true` flips the icon/colour to the danger variant.
 */
export const useToastStore = defineStore('toast', () => {
  const visible = ref(false)
  const message = ref('')
  const isError = ref(false)
  let timer: ReturnType<typeof setTimeout> | null = null

  function show(msg: string, opts: { error?: boolean; duration?: number } = {}): void {
    message.value = msg
    isError.value = opts.error ?? false
    visible.value = true
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      visible.value = false
    }, opts.duration ?? 3500)
  }

  function hide(): void {
    visible.value = false
    if (timer) {
      clearTimeout(timer)
      timer = null
    }
  }

  return { visible, message, isError, show, hide }
})
