<script setup lang="ts">
import { formatCents } from '../../lib/format'
import type { BudgetMonthCategory } from '../../types'

defineProps<{
  envelopes: BudgetMonthCategory[]
}>()
</script>

<template>
  <div class="rounded-card border border-border bg-card p-5 shadow-card">
    <div class="mb-[14px] flex items-center justify-between">
      <h2 class="text-[16px] font-semibold text-primary leading-none">Overspent envelopes</h2>
      <router-link to="/plan" class="text-[13px] font-semibold text-accent hover:underline">
        Go to Plan
      </router-link>
    </div>

    <!-- Overspent list -->
    <ul v-if="envelopes.length > 0" class="flex flex-col gap-[10px]">
      <li
        v-for="envelope in envelopes"
        :key="envelope.categoryId"
        class="flex items-center justify-between rounded-[10px] bg-red/[0.07] border border-red/25 border-l-[3px] border-l-red px-[14px] py-[11px]"
      >
        <span class="text-[13px] font-medium text-primary">{{ envelope.name }}</span>
        <span class="num text-[13px] font-semibold text-red">{{
          formatCents(envelope.available)
        }}</span>
      </li>
    </ul>

    <!-- On-budget state -->
    <div v-else class="rounded-[10px] bg-green/10 px-[14px] py-[14px] text-center">
      <p class="text-[13px] font-semibold text-green">You're on budget — no overspent envelopes.</p>
    </div>
  </div>
</template>
