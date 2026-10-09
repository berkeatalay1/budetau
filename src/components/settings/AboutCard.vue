<script setup lang="ts">
import { onMounted, ref } from 'vue'

// Fallback shown outside the Tauri runtime (e.g. plain-browser mock
// verification) or if the `plugin:app|version` call ever fails.
const FALLBACK_VERSION = '—'

const version = ref(FALLBACK_VERSION)

onMounted(async () => {
  try {
    const { getVersion } = await import('@tauri-apps/api/app')
    version.value = await getVersion()
  } catch {
    version.value = FALLBACK_VERSION
  }
})
</script>

<template>
  <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
    <div class="flex items-center gap-3.5">
      <!-- App glyph — matches the sidebar wordmark -->
      <div class="relative h-10 w-10 flex-none rounded-control bg-accent" aria-hidden="true">
        <div
          class="absolute rounded-full border-[3px] border-white"
          style="width: 16px; height: 16px; left: 12px; top: 12px"
        ></div>
      </div>

      <div class="min-w-0 flex-1">
        <p class="text-[15px] font-semibold leading-snug text-primary">Budetau</p>
        <p class="text-[13px] leading-relaxed text-secondary">
          Local-first envelope budgeting — your data never leaves this Mac.
        </p>
      </div>
    </div>

    <div class="mt-4 flex items-center gap-2 border-t border-[#eef0f3] dark:border-border pt-4">
      <span class="text-[12px] font-medium text-secondary">Version {{ version }}</span>
    </div>
  </div>
</template>
