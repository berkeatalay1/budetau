<script setup lang="ts">
import { computed } from 'vue'
import { formatCents } from '../../lib/format'

const props = defineProps<{
  name: string
  colorCode: string | null
  spent: number
  limit: number
  alertThreshold: number
}>()

type Status = 'under' | 'near' | 'over'

const pct = computed(() => (props.limit > 0 ? props.spent / props.limit : 0))
const pctDisplay = computed(() => Math.round(pct.value * 100))
const barWidth = computed(() => Math.min(100, pctDisplay.value) + '%')

const status = computed<Status>(() => {
  if (pct.value >= 1) return 'over'
  if (pct.value >= props.alertThreshold) return 'near'
  return 'under'
})

const remaining = computed(() => props.limit - props.spent)

// Token classes by status
const badgeClass = computed(() => {
  if (status.value === 'over') return 'text-red bg-red/10'
  if (status.value === 'near') return 'text-amber bg-amber/10'
  return 'text-green bg-green/10'
})

const barClass = computed(() => {
  if (status.value === 'over') return 'bg-red'
  if (status.value === 'near') return 'bg-amber'
  return 'bg-green'
})

const footerClass = computed(() => {
  if (status.value === 'over') return 'text-red'
  if (status.value === 'near') return 'text-amber'
  return 'text-green'
})

// Card border/background: near and over get a tinted border like artboard 08
const cardClass = computed(() => {
  if (status.value === 'over') return 'border-red/30 bg-red/[0.02]'
  if (status.value === 'near') return 'border-amber/40 bg-amber/[0.02]'
  return 'border-border bg-card'
})

const footerText = computed(() => {
  if (status.value === 'over') {
    const over = props.spent - props.limit
    return `● ${formatCents(over)} over budget`
  }
  if (status.value === 'near') {
    if (remaining.value === 0) return `$0 left · at limit`
    return `⚠ ${formatCents(remaining.value)} left · near limit`
  }
  return `${formatCents(remaining.value)} remaining`
})
</script>

<template>
  <div class="rounded-card border p-4 shadow-card" :class="cardClass">
    <!-- Header: dot + name + badge -->
    <div class="flex items-center justify-between mb-[14px]">
      <div class="flex items-center gap-2">
        <span
          class="w-[10px] h-[10px] rounded-full flex-none"
          :style="colorCode ? { background: colorCode } : { background: 'rgb(var(--c-secondary))' }"
        ></span>
        <span class="font-semibold text-sm text-primary leading-none">{{ name }}</span>
      </div>
      <span
        class="text-[11px] font-semibold px-2 py-1 rounded-pill leading-none"
        :class="badgeClass"
      >
        {{ pctDisplay }}%
      </span>
    </div>

    <!-- Amounts -->
    <div class="flex items-baseline gap-[5px] mb-[10px]">
      <span class="num font-bold text-h2 text-primary leading-none">
        {{ formatCents(spent) }}
      </span>
      <span class="num text-[13px] font-medium text-secondary leading-none">
        of {{ formatCents(limit) }}
      </span>
    </div>

    <!-- Progress bar -->
    <div class="h-2 bg-border/40 rounded-pill overflow-hidden">
      <div
        class="h-full rounded-pill transition-[width]"
        :class="barClass"
        :style="{ width: barWidth }"
      ></div>
    </div>

    <!-- Footer -->
    <div class="text-[12px] font-medium mt-[10px] leading-none" :class="footerClass">
      {{ footerText }}
    </div>
  </div>
</template>
