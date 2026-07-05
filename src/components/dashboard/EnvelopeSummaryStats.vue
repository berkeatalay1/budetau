<script setup lang="ts">
import { formatCents } from '../../lib/format'

defineProps<{
  totalAssigned: number
  totalActivity: number
  totalAvailable: number
}>()
</script>

<template>
  <div class="mb-[18px] grid grid-cols-3 gap-[14px]">
    <!-- Assigned -->
    <div class="rounded-card border border-border bg-card px-5 py-4 shadow-card">
      <div class="text-caption uppercase tracking-[.05em] text-secondary mb-[10px]">Assigned</div>
      <div class="num text-[22px] font-bold leading-none tracking-[-0.01em] text-primary">
        {{ formatCents(totalAssigned) }}
      </div>
    </div>

    <!-- Activity -->
    <div class="rounded-card border border-border bg-card px-5 py-4 shadow-card">
      <div class="text-caption uppercase tracking-[.05em] text-secondary mb-[10px]">Activity</div>
      <div class="num text-[22px] font-bold leading-none tracking-[-0.01em] text-primary">
        {{ formatCents(totalActivity) }}
      </div>
    </div>

    <!-- Available -->
    <div class="rounded-card border border-border bg-card px-5 py-4 shadow-card">
      <div class="text-caption uppercase tracking-[.05em] text-secondary mb-[10px]">Available</div>
      <div
        class="num text-[22px] font-bold leading-none tracking-[-0.01em]"
        :class="totalAvailable < 0 ? 'text-red' : 'text-primary'"
      >
        {{ formatCents(totalAvailable) }}
      </div>
    </div>
  </div>
</template>
