<script setup lang="ts">
defineProps<{
  title: string
  message: string
  confirmLabel?: string
  danger?: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-primary/30 backdrop-blur-sm"
    @click.self="emit('cancel')"
  >
    <div
      class="relative w-full max-w-sm rounded-card bg-card shadow-pop border border-border p-6"
      role="dialog"
      aria-modal="true"
    >
      <h2 class="text-h2 text-primary mb-2">{{ title }}</h2>
      <p class="text-body text-secondary mb-6">{{ message }}</p>

      <div class="flex justify-end gap-3">
        <button
          class="rounded-control border border-border bg-card px-4 py-2 text-caption text-secondary hover:bg-bg transition-colors"
          @click="emit('cancel')"
        >
          Cancel
        </button>
        <button
          :class="[
            'rounded-control px-4 py-2 text-caption font-semibold transition-colors',
            danger
              ? 'bg-red text-white hover:bg-red/90'
              : 'bg-accent text-white hover:bg-accent/90',
          ]"
          @click="emit('confirm')"
        >
          {{ confirmLabel ?? 'Confirm' }}
        </button>
      </div>
    </div>
  </div>
</template>
