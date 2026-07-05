<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useBudgetMonthStore } from '../stores/budgetMonth'
import { useCategoryStore } from '../stores/category'
import type { BudgetMonthCategory, Category } from '../types'
import { formatCents } from '../lib/format'
import EnvelopeRow from '../components/plan/EnvelopeRow.vue'
import MoveMoneyModal from '../components/plan/MoveMoneyModal.vue'

const budgetMonthStore = useBudgetMonthStore()
const categoryStore = useCategoryStore()

onMounted(async () => {
  const tasks: Promise<unknown>[] = [budgetMonthStore.fetch()]
  if (categoryStore.data.length === 0) tasks.push(categoryStore.fetchAll())
  await Promise.all(tasks)
})

// ── Month navigation ──────────────────────────────────────────────────────────
/** Add `delta` months to a 'YYYY-MM' string, returning a new 'YYYY-MM' string. */
function shiftMonth(month: string, delta: number): string {
  const [y, m] = month.split('-').map(Number)
  const d = new Date(y, m - 1 + delta, 1)
  const year = d.getFullYear()
  const mo = String(d.getMonth() + 1).padStart(2, '0')
  return `${year}-${mo}`
}

const monthLabel = computed(() => {
  const [y, m] = budgetMonthStore.month.split('-').map(Number)
  return new Date(y, m - 1, 1).toLocaleDateString('en-US', { month: 'long', year: 'numeric' })
})

function goPrevMonth() {
  budgetMonthStore.setMonth(shiftMonth(budgetMonthStore.month, -1))
}

function goNextMonth() {
  budgetMonthStore.setMonth(shiftMonth(budgetMonthStore.month, 1))
}

// ── Ready to Assign header state ──────────────────────────────────────────────
const rta = computed(() => budgetMonthStore.data?.readyToAssign ?? 0)

const rtaState = computed<'zero' | 'positive' | 'negative'>(() => {
  if (rta.value === 0) return 'zero'
  return rta.value > 0 ? 'positive' : 'negative'
})

const rtaLabel = computed(() => {
  if (rtaState.value === 'zero') return 'All money assigned'
  if (rtaState.value === 'positive') return 'Ready to assign'
  return 'Over-assigned'
})

const rtaClasses = computed(() => {
  switch (rtaState.value) {
    case 'positive':
      return 'bg-accent/10 text-accent'
    case 'negative':
      return 'bg-red/10 text-red'
    default:
      return 'bg-green/10 text-green'
  }
})

// ── Category rows: build a lookup, then group parent → children ──────────────
const rowsByCategory = computed<Map<number, BudgetMonthCategory>>(
  () => new Map((budgetMonthStore.data?.categories ?? []).map((c) => [c.categoryId, c])),
)

const expenseTopLevel = computed<Category[]>(() =>
  categoryStore.topLevel.filter((c) => c.type === 'expense'),
)

/** All expense categories (top-level + sub), flattened — each is its own
 * envelope and a valid move-money endpoint. */
const allExpenseCategories = computed<Category[]>(() =>
  categoryStore.data.filter((c) => c.type === 'expense'),
)

function rowFor(categoryId: number): BudgetMonthCategory {
  return (
    rowsByCategory.value.get(categoryId) ?? {
      categoryId,
      name: categoryStore.byId.get(categoryId)?.name ?? 'Unknown',
      assigned: 0,
      activity: 0,
      available: 0,
    }
  )
}

interface GroupTotal {
  assigned: number
  activity: number
  available: number
}

function groupTotalFor(parentId: number): GroupTotal {
  const own = rowFor(parentId)
  const children = categoryStore.childrenOf(parentId)
  return children.reduce(
    (acc, child) => {
      const row = rowFor(child.id)
      return {
        assigned: acc.assigned + row.assigned,
        activity: acc.activity + row.activity,
        available: acc.available + row.available,
      }
    },
    { assigned: own.assigned, activity: own.activity, available: own.available },
  )
}

// ── Inline-editable Assigned cell ─────────────────────────────────────────────
async function commitAssign(categoryId: number, rawValue: string) {
  const dollars = Number.parseFloat(rawValue)
  if (Number.isNaN(dollars)) return
  const cents = Math.round(dollars * 100)
  await budgetMonthStore.assign(categoryId, cents)
}

// ── Move-money modal ───────────────────────────────────────────────────────────
const moveMoneyFrom = ref<number | null>(null)
const showMoveMoney = ref(false)

function openMoveMoney(categoryId: number) {
  moveMoneyFrom.value = categoryId
  showMoveMoney.value = true
}

function openMoveMoneyDefault() {
  moveMoneyFrom.value = expenseTopLevel.value[0]?.id ?? null
  showMoveMoney.value = true
}

function closeMoveMoney() {
  showMoveMoney.value = false
}

const isLoading = computed(() => budgetMonthStore.isLoading || categoryStore.isLoading)
const isEmpty = computed(() => !isLoading.value && expenseTopLevel.value.length === 0)
</script>

<template>
  <div class="px-7 py-6 pb-8">
    <!-- Page header -->
    <div class="mb-5 flex items-center justify-between">
      <div>
        <h1 class="text-h1 text-primary tracking-tight">Plan</h1>
        <p class="mt-[3px] text-[13px] text-secondary">Give every dollar a job</p>
      </div>

      <div class="flex items-center gap-3">
        <!-- Move money -->
        <button
          type="button"
          class="rounded-control border border-border bg-card px-3.5 py-2 text-[13px] font-semibold text-primary transition-colors hover:bg-bg"
          @click="openMoveMoneyDefault"
        >
          Move money
        </button>

        <!-- Month navigator -->
        <div
          class="flex items-center gap-2 rounded-pill border border-border bg-card px-2 py-[6px]"
        >
          <button
            class="flex h-6 w-6 items-center justify-center rounded-full text-[14px] font-semibold text-secondary transition-colors hover:text-primary"
            aria-label="Previous month"
            @click="goPrevMonth"
          >
            ‹
          </button>
          <span class="min-w-[110px] text-center text-[13px] font-semibold text-primary">
            {{ monthLabel }}
          </span>
          <button
            class="flex h-6 w-6 items-center justify-center rounded-full text-[14px] font-semibold text-secondary transition-colors hover:text-primary"
            aria-label="Next month"
            @click="goNextMonth"
          >
            ›
          </button>
        </div>
      </div>
    </div>

    <!-- Sticky Ready to Assign header -->
    <div
      class="sticky top-0 z-10 mb-5 flex items-center justify-between rounded-card border border-[#eef0f3] px-5 py-4 shadow-card dark:border-border"
      :class="rtaClasses"
    >
      <span class="text-[13px] font-semibold">{{ rtaLabel }}</span>
      <span class="num text-[22px] font-bold tabular-nums">{{ formatCents(rta) }}</span>
    </div>

    <!-- Loading -->
    <div v-if="isLoading" class="flex h-64 items-center justify-center">
      <div
        class="h-6 w-6 animate-spin rounded-full border-2 border-accent border-t-transparent"
      ></div>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="isEmpty"
      class="rounded-card border border-dashed border-border bg-card py-14 text-center shadow-card"
    >
      <p class="mb-1 text-[14px] font-semibold text-primary">No expense categories yet</p>
      <p class="text-[13px] text-secondary">
        Head to
        <router-link to="/settings" class="font-semibold text-accent hover:underline"
          >Settings → Categories</router-link
        >
        to create your first envelope.
      </p>
    </div>

    <!-- Envelope table -->
    <div
      v-else
      class="overflow-hidden rounded-card border border-[#eef0f3] bg-card shadow-card dark:border-border"
    >
      <!-- Column headers -->
      <div
        class="grid grid-cols-[minmax(160px,1fr)_110px_110px_110px_36px] gap-3 border-b border-border bg-bg px-5 py-2.5 text-[11px] font-semibold uppercase tracking-wide text-secondary"
      >
        <span>Category</span>
        <span class="text-right">Assigned</span>
        <span class="text-right">Activity</span>
        <span class="text-right">Available</span>
        <span></span>
      </div>

      <template v-for="parent in expenseTopLevel" :key="parent.id">
        <!-- Parent (roll-up) row -->
        <EnvelopeRow
          :category-id="parent.id"
          :name="parent.name"
          :row="groupTotalFor(parent.id)"
          :editable-assigned="rowFor(parent.id).assigned"
          :is-parent="true"
          @commit-assign="commitAssign"
          @move-money="openMoveMoney"
        />

        <!-- Children (indented) -->
        <EnvelopeRow
          v-for="child in categoryStore.childrenOf(parent.id)"
          :key="child.id"
          :category-id="child.id"
          :name="child.name"
          :row="rowFor(child.id)"
          :editable-assigned="rowFor(child.id).assigned"
          :indent="true"
          @commit-assign="commitAssign"
          @move-money="openMoveMoney"
        />
      </template>
    </div>
  </div>

  <!-- Move money modal -->
  <MoveMoneyModal
    v-if="showMoveMoney"
    :default-from-id="moveMoneyFrom"
    :categories="allExpenseCategories"
    @close="closeMoveMoney"
  />
</template>
