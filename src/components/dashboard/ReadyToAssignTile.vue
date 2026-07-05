<script setup lang="ts">
import { computed } from 'vue'
import { formatCents } from '../../lib/format'

const props = defineProps<{
  readyToAssign: number
}>()

const state = computed<'zero' | 'positive' | 'negative'>(() => {
  if (props.readyToAssign === 0) return 'zero'
  return props.readyToAssign > 0 ? 'positive' : 'negative'
})

const label = computed(() => {
  if (state.value === 'zero') return 'All money assigned'
  if (state.value === 'positive') return 'Ready to assign'
  return 'Over-assigned'
})

const classes = computed(() => {
  switch (state.value) {
    case 'positive':
      return 'bg-accent/10 text-accent'
    case 'negative':
      return 'bg-red/10 text-red'
    default:
      return 'bg-green/10 text-green'
  }
})
</script>

<template>
  <div
    class="mb-[18px] flex items-center justify-between rounded-card border border-[#eef0f3] px-5 py-4 shadow-card dark:border-border"
    :class="classes"
  >
    <span class="text-[13px] font-semibold">{{ label }}</span>
    <span class="num text-[22px] font-bold tabular-nums">{{ formatCents(readyToAssign) }}</span>
  </div>
</template>
