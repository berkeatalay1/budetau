<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useTransactionStore } from '../stores/transaction'
import { useBudgetStore } from '../stores/budget'
import { useCategoryStore } from '../stores/category'
import { useQuickAddStore } from '../stores/quickAdd'
import HeroSummary from '../components/dashboard/HeroSummary.vue'
import BudgetCard from '../components/dashboard/BudgetCard.vue'
import AlertBanner from '../components/dashboard/AlertBanner.vue'
import AddBudgetModal from '../components/dashboard/AddBudgetModal.vue'

// ─── stores ───────────────────────────────────────────────────────────────────
const txStore = useTransactionStore()
const budgetStore = useBudgetStore()
const categoryStore = useCategoryStore()
const quickAdd = useQuickAddStore()

// ─── budget creation modal ─────────────────────────────────────────────────────
const showAddBudget = ref(false)

// ─── month navigation ─────────────────────────────────────────────────────────
const today = new Date()
const viewYear = ref(today.getFullYear())
const viewMonth = ref(today.getMonth()) // 0-indexed

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

// ─── budget card data ─────────────────────────────────────────────────────────
interface BudgetRow {
  id: number
  name: string
  colorCode: string | null
  spent: number
  limit: number
  pct: number
  status: 'under' | 'near' | 'over'
}

const budgetRows = computed<BudgetRow[]>(() => {
  return budgetStore.data.map((b) => {
    const category = categoryStore.byId.get(b.categoryId)
    const spent = monthTransactions.value
      .filter((tx) => tx.type === 'expense' && tx.categoryId === b.categoryId)
      .reduce((sum, tx) => sum + tx.amount, 0)
    const pct = b.amount > 0 ? spent / b.amount : 0
    const status: 'under' | 'near' | 'over' = pct >= 1 ? 'over' : pct >= 0.9 ? 'near' : 'under'
    return {
      id: b.id,
      name: category?.name ?? 'Unknown',
      colorCode: category?.colorCode ?? null,
      spent,
      limit: b.amount,
      pct,
      status,
    }
  })
})

// ─── alert banners (near ≥ 90% or over ≥ 100%) ───────────────────────────────
const alerts = computed(() =>
  budgetRows.value.filter(
    (b): b is BudgetRow & { status: 'near' | 'over' } => b.status === 'near' || b.status === 'over',
  ),
)

// ─── empty state ──────────────────────────────────────────────────────────────
const isEmpty = computed(
  () =>
    !txStore.isLoading &&
    !budgetStore.isLoading &&
    txStore.data.length === 0 &&
    budgetStore.data.length === 0,
)

// ─── loading ─────────────────────────────────────────────────────────────────
const isLoading = computed(
  () => txStore.isLoading || budgetStore.isLoading || categoryStore.isLoading,
)

// ─── fetch on mount ───────────────────────────────────────────────────────────
onMounted(async () => {
  await Promise.all([txStore.fetchAll(), budgetStore.fetchAll(), categoryStore.fetchAll()])
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
        <button
          class="font-semibold text-[13px] text-primary bg-card border border-border px-[18px] py-[11px] rounded-control cursor-pointer hover:bg-bg transition-colors"
          @click="showAddBudget = true"
        >
          Set a budget
        </button>
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

    <!-- Alert banners -->
    <div v-if="alerts.length > 0" class="flex flex-col gap-[10px] mb-[22px]">
      <AlertBanner
        v-for="b in alerts"
        :key="b.id"
        :category-name="b.name"
        :spent="b.spent"
        :limit="b.limit"
        :status="b.status"
      />
    </div>

    <!-- Budgets section -->
    <div v-if="budgetRows.length > 0">
      <div class="flex items-center justify-between mb-[14px]">
        <h2 class="text-[16px] font-semibold text-primary leading-none">Budgets</h2>
        <button
          class="text-[13px] font-semibold text-accent cursor-pointer hover:underline"
          @click="showAddBudget = true"
        >
          Manage budgets
        </button>
      </div>

      <div class="grid grid-cols-3 gap-[14px]">
        <BudgetCard
          v-for="b in budgetRows"
          :key="b.id"
          :name="b.name"
          :color-code="b.colorCode"
          :spent="b.spent"
          :limit="b.limit"
        />
      </div>
    </div>

    <!-- No budgets yet (but has transactions) -->
    <div v-else class="rounded-card border border-border bg-card p-8 text-center shadow-card mt-2">
      <p class="text-secondary text-body">No budgets set yet.</p>
      <button
        class="mt-4 font-semibold text-[13px] text-accent hover:underline cursor-pointer"
        @click="showAddBudget = true"
      >
        Set a budget
      </button>
    </div>
  </div>

  <!-- Budget creation modal — available from empty state, header link, and no-budgets card -->
  <AddBudgetModal :open="showAddBudget" @close="showAddBudget = false" />
</template>
