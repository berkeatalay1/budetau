<script setup lang="ts">
import { useQuickAddStore } from '../stores/quickAdd'

const quickAdd = useQuickAddStore()
</script>

<template>
  <!-- Floating action button — bottom-right, accent circle with a "+" cross -->
  <button
    type="button"
    aria-label="Add transaction"
    class="absolute bottom-[22px] right-[22px] z-10 flex h-[50px] w-[50px] items-center justify-center rounded-full bg-accent shadow-[0_6px_18px_rgba(37,99,235,.45)] transition-transform duration-150 hover:scale-105 active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2"
    @click="quickAdd.open()"
  >
    <!-- "+" icon built from two rectangles -->
    <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
      <rect x="0" y="7.75" width="18" height="2.5" rx="1.25" fill="white" />
      <rect x="7.75" y="0" width="2.5" height="18" rx="1.25" fill="white" />
    </svg>
  </button>
</template>
