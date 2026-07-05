<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useTransactionStore } from '../stores/transaction'
import { useCategoryStore } from '../stores/category'
import { useQuickAddStore } from '../stores/quickAdd'
import { getBudgetMonth } from '../api'
import type { BudgetMonth, BudgetMonthCategory } from '../types'
import HeroSummary from '../components/dashboard/HeroSummary.vue'
import ReadyToAssignTile from '../components/dashboard/ReadyToAssignTile.vue'
import EnvelopeSummaryStats from '../components/dashboard/EnvelopeSummaryStats.vue'
import OverspentEnvelopes from '../components/dashboard/OverspentEnvelopes.vue'

// ─── stores ───────────────────────────────────────────────────────────────────
const txStore = useTransactionStore()
const categoryStore = useCategoryStore()
const quickAdd = useQuickAddStore()

// ─── month navigation ─────────────────────────────────────────────────────────
const today = new Date()
const viewYear = ref(today.getFullYear())
const viewMonth = ref(today.getMonth()) // 0-indexed

/** Convert the 0-indexed viewYear/viewMonth into a 'YYYY-MM' string. */
const viewMonthKey = computed(() => {
  const mo = String(viewMonth.value + 1).padStart(2, '0')
  return `${viewYear.value}-${mo}`
})

function prevMonth() {
  if (viewMonth.value === 0) {
    viewMonth.value = 11
    viewYear.value--
  } else {
    viewMonth.value--
  }
}
function nextMonth() {
  if (viewMonth.value === 11) {
    viewMonth.value = 0
    viewYear.value++
  } else {
    viewMonth.value++
  }
}

const monthLabel = computed(() =>
  new Date(viewYear.value, viewMonth.value, 1).toLocaleDateString('en-US', {
    month: 'long',
    year: 'numeric',
  }),
)

// ─── transactions filtered to the selected month ──────────────────────────────
const monthTransactions = computed(() => {
  const y = viewYear.value
  const m = viewMonth.value
  return txStore.data.filter((tx) => {
    const d = new Date(tx.date + 'T00:00:00')
    return d.getFullYear() === y && d.getMonth() === m
  })
})

// ─── hero aggregates ──────────────────────────────────────────────────────────
const totalIncome = computed(() =>
  monthTransactions.value
    .filter((tx) => tx.type === 'income')
    .reduce((sum, tx) => sum + tx.amount, 0),
)

const totalSpend = computed(() =>
  monthTransactions.value
    .filter((tx) => tx.type === 'expense')
    .reduce((sum, tx) => sum + tx.amount, 0),
)

// ─── envelope data (v5 read model) ────────────────────────────────────────────
// Fetched directly via the API into a LOCAL ref — the Dashboard must not call
// budgetMonthStore.setMonth(...), since that store is a singleton shared with
// the Plan screen and mutating its month would change what Plan shows next.
const monthData = ref<BudgetMonth | null>(null)
const isBudgetMonthLoading = ref(false)

async function fetchMonthData() {
  isBudgetMonthLoading.value = true
  try {
    monthData.value = await getBudgetMonth(viewMonthKey.value)
  } catch {
    monthData.value = null
  } finally {
    isBudgetMonthLoading.value = false
  }
}

watch(viewMonthKey, () => {
  fetchMonthData()
})

const readyToAssign = computed(() => monthData.value?.readyToAssign ?? 0)

const categories = computed<BudgetMonthCategory[]>(() => monthData.value?.categories ?? [])

const totalAssigned = computed(() => categories.value.reduce((sum, c) => sum + c.assigned, 0))
const totalActivity = computed(() => categories.value.reduce((sum, c) => sum + c.activity, 0))
const totalAvailable = computed(() => categories.value.reduce((sum, c) => sum + c.available, 0))

const overspentEnvelopes = computed(() =>
  categories.value
    .filter((c) => c.available < 0)
    .sort((a, b) => a.available - b.available)
    .slice(0, 5),
)

// ─── empty state ──────────────────────────────────────────────────────────────
const isEmpty = computed(
  () => !txStore.isLoading && txStore.data.length === 0 && categories.value.length === 0,
)

// ─── loading ─────────────────────────────────────────────────────────────────
const isLoading = computed(
  () => txStore.isLoading || categoryStore.isLoading || isBudgetMonthLoading.value,
)

// ─── fetch on mount ───────────────────────────────────────────────────────────
onMounted(async () => {
  await Promise.all([txStore.fetchAll(), categoryStore.fetchAll(), fetchMonthData()])
})
</script>

<template>
  <!-- Loading skeleton -->
  <div v-if="isLoading" class="flex items-center justify-center h-64">
    <div
      class="w-6 h-6 rounded-full border-2 border-accent border-t-transparent animate-spin"
    ></div>
  </div>

  <!-- Empty / first-run state -->
  <div v-else-if="isEmpty" class="flex flex-1 items-center justify-center py-16">
    <div class="text-center max-w-[380px]">
      <!-- Icon block -->
      <div
        class="w-[72px] h-[72px] rounded-[18px] bg-accent/10 flex items-center justify-center mx-auto mb-[22px]"
      >
        <div class="w-[30px] h-[30px] grid grid-cols-2 gap-1">
          <div class="bg-accent/30 rounded-[3px]"></div>
          <div class="bg-accent/30 rounded-[3px]"></div>
          <div class="bg-accent/30 rounded-[3px]"></div>
          <div class="bg-accent rounded-[3px]"></div>
        </div>
      </div>

      <h2 class="text-h1 text-primary mb-2">Welcome to Budetau</h2>
      <p class="text-body text-secondary mb-6">
        Your money lives in one private file on this Mac — no accounts, no cloud. Log your first
        transaction to start seeing your month at a glance.
      </p>

      <div class="flex gap-[10px] justify-center">
        <button
          class="inline-flex items-center gap-2 bg-accent text-card font-semibold text-[13px] px-[18px] py-[11px] rounded-control border-none cursor-pointer hover:bg-accent/90 transition-colors"
          @click="quickAdd.open('expense')"
        >
          <span class="text-[15px] leading-none">+</span>
          Add a transaction
        </button>
        <router-link
          to="/plan"
          class="font-semibold text-[13px] text-primary bg-card border border-border px-[18px] py-[11px] rounded-control cursor-pointer hover:bg-bg transition-colors"
        >
          Head to Plan to give your money a job
        </router-link>
      </div>

      <p class="text-caption text-secondary mt-[18px]">
        Tip: press
        <kbd
          class="bg-border/50 border border-border rounded-[5px] px-[6px] py-[2px] font-semibold text-secondary"
          >⌘K</kbd
        >
        anytime to add
      </p>
    </div>
  </div>

  <!-- Populated state -->
  <div v-else class="px-7 py-6">
    <!-- Page header + month nav -->
    <div class="flex items-center justify-between mb-5">
      <div>
        <h1 class="text-h1 text-primary tracking-[-0.01em]">Dashboard</h1>
        <p class="text-[13px] text-secondary mt-[3px]">Your month at a glance</p>
      </div>
      <!-- Month navigator -->
      <div class="flex items-center gap-2 bg-card border border-border rounded-pill px-2 py-[6px]">
        <button
          class="w-6 h-6 rounded-full flex items-center justify-center text-secondary font-semibold text-[14px] hover:text-primary transition-colors"
          aria-label="Previous month"
          @click="prevMonth"
        >
          ‹
        </button>
        <span class="font-semibold text-[13px] text-primary min-w-[80px] text-center">
          {{ monthLabel }}
        </span>
        <button
          class="w-6 h-6 rounded-full flex items-center justify-center text-secondary font-semibold text-[14px] hover:text-primary transition-colors"
          aria-label="Next month"
          @click="nextMonth"
        >
          ›
        </button>
      </div>
    </div>

    <!-- Hero summary -->
    <HeroSummary :income="totalIncome" :spend="totalSpend" :month-label="monthLabel" />

    <!-- Ready to Assign -->
    <ReadyToAssignTile :ready-to-assign="readyToAssign" />

    <!-- Assigned vs Available summary -->
    <EnvelopeSummaryStats
      :total-assigned="totalAssigned"
      :total-activity="totalActivity"
      :total-available="totalAvailable"
    />

    <!-- Top overspent envelopes -->
    <OverspentEnvelopes :envelopes="overspentEnvelopes" />
  </div>
</template>
