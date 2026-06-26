<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useTransactionStore } from '../stores/transaction'
import { useCategoryStore } from '../stores/category'
import CashFlowChart from '../components/analytics/CashFlowChart.vue'
import SpendingDonutChart from '../components/analytics/SpendingDonutChart.vue'
import MonthOverMonthChart from '../components/analytics/MonthOverMonthChart.vue'

const txStore = useTransactionStore()
const categoryStore = useCategoryStore()

const isLoading = computed(() => txStore.isLoading || categoryStore.isLoading)
const isEmpty = computed(() => !isLoading.value && txStore.data.length === 0)

onMounted(async () => {
  await Promise.all([txStore.fetchAll(), categoryStore.fetchAll()])
})

const currentMonthLabel = computed(() =>
  new Date().toLocaleString('en-US', { month: 'long', year: 'numeric' }),
)
</script>

<template>
  <!-- Loading -->
  <div v-if="isLoading" class="flex items-center justify-center h-64">
    <div
      class="w-6 h-6 rounded-full border-2 border-accent border-t-transparent animate-spin"
    ></div>
  </div>

  <!-- Empty state -->
  <div v-else-if="isEmpty" class="flex flex-1 items-center justify-center py-16">
    <div class="text-center max-w-[360px]">
      <!-- Icon -->
      <div
        class="w-[72px] h-[72px] rounded-[18px] bg-accent/10 flex items-center justify-center mx-auto mb-[22px]"
      >
        <svg
          width="32"
          height="32"
          viewBox="0 0 32 32"
          fill="none"
          aria-hidden="true"
          class="text-accent"
        >
          <rect x="2" y="20" width="6" height="10" rx="2" fill="currentColor" opacity="0.4" />
          <rect x="13" y="12" width="6" height="18" rx="2" fill="currentColor" opacity="0.65" />
          <rect x="24" y="4" width="6" height="26" rx="2" fill="currentColor" />
        </svg>
      </div>
      <h2 class="text-h1 text-primary mb-2">No data yet</h2>
      <p class="text-body text-secondary">
        Add some transactions to start seeing charts and insights about your spending.
      </p>
    </div>
  </div>

  <!-- Populated -->
  <div v-else class="px-7 py-6">
    <!-- Page header -->
    <div class="mb-6">
      <h1 class="text-h1 text-primary tracking-[-0.01em]">Analytics</h1>
      <p class="text-[13px] text-secondary mt-[3px]">Insights into your spending</p>
    </div>

    <!-- Charts grid -->
    <div class="flex flex-col gap-5">
      <!-- Row 1: Cash flow (full width) -->
      <div class="rounded-card border border-border bg-card shadow-card p-5">
        <div class="mb-4">
          <h2 class="text-[15px] font-semibold text-primary leading-none">Cash flow</h2>
          <p class="text-[12px] text-secondary mt-[3px]">Income vs expenses — last 6 months</p>
        </div>
        <CashFlowChart :transactions="txStore.data" />
      </div>

      <!-- Row 2: Donut + Bar side by side -->
      <div class="grid grid-cols-1 min-[760px]:grid-cols-2 gap-5">
        <!-- Spending by category -->
        <div class="rounded-card border border-border bg-card shadow-card p-5">
          <div class="mb-1">
            <h2 class="text-[15px] font-semibold text-primary leading-none">
              Spending by category
            </h2>
            <p class="text-[12px] text-secondary mt-[3px]">{{ currentMonthLabel }}</p>
          </div>
          <SpendingDonutChart :transactions="txStore.data" :category-by-id="categoryStore.byId" />
        </div>

        <!-- Month-over-month net -->
        <div class="rounded-card border border-border bg-card shadow-card p-5">
          <div class="mb-4">
            <h2 class="text-[15px] font-semibold text-primary leading-none">Monthly net</h2>
            <p class="text-[12px] text-secondary mt-[3px]">Income minus expenses — last 6 months</p>
          </div>
          <MonthOverMonthChart :transactions="txStore.data" />
        </div>
      </div>
    </div>
  </div>
</template>
