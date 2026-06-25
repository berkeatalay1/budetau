<script setup lang="ts">
import { computed } from 'vue'
import { formatCents } from '../../lib/format'

const props = defineProps<{
  income: number
  spend: number
  monthLabel: string
}>()

const net = computed(() => props.income - props.spend)
const spendPct = computed(() =>
  props.income > 0 ? Math.min(100, Math.round((props.spend / props.income) * 100)) : 0,
)
const netIsPositive = computed(() => net.value >= 0)
</script>

<template>
  <div class="rounded-card border border-border bg-card px-6 py-[22px] shadow-card mb-[18px]">
    <div class="flex items-stretch">
      <!-- Net -->
      <div class="flex-[1.2] pr-6">
        <div class="text-caption uppercase tracking-[.05em] text-secondary mb-[10px]">
          Net this month
        </div>
        <div
          class="num font-bold leading-none tracking-[-0.02em]"
          :class="netIsPositive ? 'text-green text-[38px]' : 'text-red text-[38px]'"
        >
          {{ netIsPositive ? '+' : '' }}{{ formatCents(net) }}
        </div>
        <div class="text-caption text-secondary mt-[10px]">
          <template v-if="netIsPositive">You're spending less than you earn</template>
          <template v-else>You're spending more than you earn</template>
        </div>
      </div>

      <!-- Divider -->
      <div class="w-px bg-border flex-none"></div>

      <!-- Income -->
      <div class="flex-1 px-6">
        <div class="text-caption uppercase tracking-[.05em] text-secondary mb-[10px]">Income</div>
        <div class="num font-bold text-[26px] leading-none tracking-[-0.01em] text-primary">
          {{ formatCents(income) }}
        </div>
      </div>

      <!-- Divider -->
      <div class="w-px bg-border flex-none"></div>

      <!-- Spent -->
      <div class="flex-1 pl-6">
        <div class="text-caption uppercase tracking-[.05em] text-secondary mb-[10px]">Spent</div>
        <div class="num font-bold text-[26px] leading-none tracking-[-0.01em] text-primary">
          {{ formatCents(spend) }}
        </div>
        <div class="text-caption text-secondary mt-2">{{ spendPct }}% of income</div>
      </div>
    </div>

    <!-- Progress bar: spend / income -->
    <div class="h-[6px] bg-border/40 rounded-pill overflow-hidden mt-[18px]">
      <div class="h-full bg-accent rounded-pill" :style="{ width: spendPct + '%' }"></div>
    </div>
  </div>
</template>
