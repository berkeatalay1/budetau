import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export type ThemeMode = 'light' | 'dark' | 'system'

const STORAGE_KEY = 'budetau.theme'

function systemPrefersDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

/**
 * Theme store — manages light / dark / system and applies the `.dark` class
 * to <html>, which flips the CSS color tokens in main.css.
 */
export const useThemeStore = defineStore('theme', () => {
  const stored = (localStorage.getItem(STORAGE_KEY) as ThemeMode | null) ?? 'system'
  const mode = ref<ThemeMode>(stored)

  function resolveDark(): boolean {
    return mode.value === 'dark' || (mode.value === 'system' && systemPrefersDark())
  }

  function apply(): void {
    document.documentElement.classList.toggle('dark', resolveDark())
  }

  function setMode(next: ThemeMode): void {
    mode.value = next
  }

  watch(
    mode,
    (next) => {
      localStorage.setItem(STORAGE_KEY, next)
      apply()
    },
    { immediate: true },
  )

  // React to OS theme changes while in `system` mode.
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (mode.value === 'system') apply()
  })

  return { mode, setMode, resolveDark }
})
