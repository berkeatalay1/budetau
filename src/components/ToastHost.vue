<script setup lang="ts">
import { useToastStore } from '../stores/toast'

// Single global toast host — mounted once in App.vue. Driven entirely by the
// toast store; call `useToastStore().show(...)` from anywhere.
const toast = useToastStore()
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 translate-y-2"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition duration-200 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 translate-y-2"
    >
      <div
        v-if="toast.visible"
        class="fixed bottom-6 left-1/2 z-[200] -translate-x-1/2 flex items-center gap-3 rounded-[10px] bg-[#1f2937] px-4 py-3 shadow-[0_8px_24px_rgba(16,24,40,.18)]"
        style="color: #fff"
        role="status"
      >
        <div
          class="flex h-5 w-5 flex-none items-center justify-center rounded-full text-xs font-bold text-card"
          :class="toast.isError ? 'bg-red' : 'bg-green'"
        >
          {{ toast.isError ? '!' : '✓' }}
        </div>
        <span class="text-[13px] font-medium" style="color: #fff">{{ toast.message }}</span>
      </div>
    </Transition>
  </Teleport>
</template>
