<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getDbPath } from '../api'

const dbPath = ref<string | null>(null)
const isLoading = ref(false)
const error = ref<string | null>(null)

onMounted(async () => {
  isLoading.value = true
  error.value = null
  try {
    dbPath.value = await getDbPath()
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    isLoading.value = false
  }
})
</script>

<template>
  <section>
    <h1 class="text-2xl font-semibold text-primary">Settings</h1>
    <p class="mt-2 text-secondary">Manage your profile, preferences, and application data.</p>

    <div class="mt-6 rounded-lg border border-border bg-card p-6 shadow-sm">
      <h2 class="text-sm font-medium uppercase tracking-wide text-secondary">Database</h2>
      <p v-if="isLoading" class="mt-2 text-secondary">Loading database path…</p>
      <p v-else-if="error" class="mt-2 text-red-500">Failed to load database path: {{ error }}</p>
      <p v-else class="mt-2 break-all font-mono text-sm text-primary">{{ dbPath }}</p>
    </div>
  </section>
</template>
