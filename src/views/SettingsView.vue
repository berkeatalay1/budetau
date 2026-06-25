<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getDbPath } from '../api'
import { useThemeStore } from '../stores/theme'
import type { ThemeMode } from '../stores/theme'

const themeStore = useThemeStore()

const dbPath = ref<string | null>(null)
const isLoading = ref(false)
const dbError = ref<string | null>(null)
const copied = ref(false)

onMounted(async () => {
  isLoading.value = true
  dbError.value = null
  try {
    dbPath.value = await getDbPath()
  } catch (err) {
    dbError.value = err instanceof Error ? err.message : String(err)
  } finally {
    isLoading.value = false
  }
})

async function copyPath() {
  if (!dbPath.value) return
  try {
    await navigator.clipboard.writeText(dbPath.value)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch {
    // clipboard not available in Tauri webview — silently skip
  }
}

const themeOptions: { value: ThemeMode; label: string }[] = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
  { value: 'system', label: 'System' },
]
</script>

<template>
  <div class="px-7 py-6 pb-8">
    <!-- Page title -->
    <div class="mb-5">
      <h1 class="text-h1 text-primary tracking-tight">Settings</h1>
    </div>

    <div class="flex max-w-[640px] flex-col gap-4">
      <!-- ── Database card ───────────────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <!-- Card header -->
        <div class="mb-1 flex items-center gap-2">
          <span class="text-[15px] font-semibold text-primary">Database</span>
          <span
            class="inline-flex items-center gap-[5px] rounded-pill bg-green/10 px-2 py-1 text-[11px] font-semibold text-green"
          >
            <span class="h-[6px] w-[6px] rounded-full bg-green"></span>
            On this Mac
          </span>
        </div>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          All your data lives in one private file. No accounts, no servers — nothing leaves your
          device.
        </p>

        <!-- File location row -->
        <div
          class="flex items-center gap-3 rounded-[9px] border border-[#eef0f3] dark:border-border bg-bg px-[14px] py-3"
        >
          <!-- DB icon -->
          <div
            class="flex h-[34px] w-[34px] flex-none items-center justify-center rounded-[8px] bg-accent/10"
          >
            <svg
              width="16"
              height="13"
              viewBox="0 0 16 13"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <rect
                x="1"
                y="1"
                width="14"
                height="11"
                rx="2.5"
                stroke="rgb(var(--c-accent))"
                stroke-width="2"
              />
            </svg>
          </div>

          <div class="min-w-0 flex-1">
            <p class="mb-[2px] text-[12px] font-medium leading-snug text-secondary">
              File location
            </p>
            <p v-if="isLoading" class="text-[13px] text-secondary">Loading…</p>
            <p v-else-if="dbError" class="text-[13px] text-red">{{ dbError }}</p>
            <p v-else class="num break-all text-[13px] font-medium leading-snug text-primary">
              {{ dbPath }}
            </p>
          </div>

          <button
            type="button"
            class="flex-none rounded-[7px] border border-border bg-card px-3 py-2 text-[12px] font-semibold text-primary whitespace-nowrap transition-colors hover:bg-bg"
            @click="copyPath"
          >
            {{ copied ? '✓ Copied' : 'Copy path' }}
          </button>
        </div>
      </div>

      <!-- ── Appearance card ─────────────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <h2 class="mb-1 text-[15px] font-semibold text-primary">Appearance</h2>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          Choose how Budetau looks. System follows your macOS setting.
        </p>

        <!-- Segmented control -->
        <div class="inline-flex gap-[2px] rounded-[10px] bg-[#f1f3f5] dark:bg-border/20 p-[3px]">
          <button
            v-for="opt in themeOptions"
            :key="opt.value"
            type="button"
            class="rounded-[8px] px-5 py-[9px] text-[13px] font-semibold transition-all"
            :class="
              themeStore.mode === opt.value
                ? 'bg-card text-primary shadow-[0_1px_2px_rgba(16,24,40,.1)]'
                : 'text-secondary hover:text-primary'
            "
            @click="themeStore.setMode(opt.value)"
          >
            {{ opt.label }}
          </button>
        </div>
      </div>

      <!-- ── Backup & restore card ──────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <h2 class="mb-1 text-[15px] font-semibold text-primary">Backup &amp; restore</h2>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          Export a timestamped copy of your database, or restore from a backup file.
        </p>

        <div class="mb-[14px] flex gap-[10px]">
          <button
            type="button"
            class="rounded-control bg-accent px-4 py-[10px] text-[13px] font-semibold text-card transition-colors hover:bg-accent/90 opacity-50 cursor-not-allowed"
            disabled
            title="Coming soon"
          >
            Export backup…
          </button>
          <button
            type="button"
            class="rounded-control border border-border bg-card px-4 py-[10px] text-[13px] font-semibold text-primary transition-colors hover:bg-bg opacity-50 cursor-not-allowed"
            disabled
            title="Coming soon"
          >
            Restore from file…
          </button>
        </div>

        <!-- Warning banner -->
        <div
          class="flex items-start gap-[10px] rounded-[9px] border border-amber/40 bg-amber/10 px-[13px] py-[11px]"
        >
          <div
            class="flex h-[18px] w-[18px] flex-none items-center justify-center rounded-full bg-amber text-[11px] font-bold text-card"
          >
            !
          </div>
          <p class="text-[12px] leading-relaxed text-amber">
            Restoring <strong class="font-semibold">overwrites</strong> your current data. Budetau
            makes an automatic backup first.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
