<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useTransactionStore } from '../stores/transaction'
import { useCategoryStore } from '../stores/category'
import { useAccountStore } from '../stores/account'
import type { Transaction } from '../types'
import { formatDayHeader, formatSignedCents } from '../lib/format'
import TransactionCard from '../components/transactions/TransactionCard.vue'
import EditTransactionModal from '../components/transactions/EditTransactionModal.vue'
import ConfirmDialog from '../components/transactions/ConfirmDialog.vue'

// ─── Stores ───────────────────────────────────────────────────────────────────
const transactionStore = useTransactionStore()
const categoryStore = useCategoryStore()
const accountStore = useAccountStore()

onMounted(async () => {
  await Promise.all([
    transactionStore.fetchAll(),
    categoryStore.fetchAll(),
    accountStore.fetchAll(),
  ])
})

// ─── Filter state ─────────────────────────────────────────────────────────────
const searchQuery = ref('')
const filterAccountId = ref<number | null>(null)
const filterType = ref<string | null>(null)
const filterMonth = ref<string>('') // 'YYYY-MM' or ''
const filterCategoryId = ref<number | null>(null) // null = all categories

// Build month options from all transaction dates
const monthOptions = computed(() => {
  const set = new Set<string>()
  for (const t of transactionStore.data) {
    set.add(t.date.slice(0, 7))
  }
  return [...set].sort().reverse()
})

// Default the month filter to the most recent month present in data
// (only on first load; user can then clear)
const hasAutoSetMonth = ref(false)
const effectiveMonth = computed(() => {
  // If user explicitly set a month (including '' = all), use it
  if (hasAutoSetMonth.value) return filterMonth.value
  // Auto-set to the latest month when data loads
  return monthOptions.value[0] ?? ''
})

function onMonthChange(val: string) {
  hasAutoSetMonth.value = true
  filterMonth.value = val
}

// ─── Category filter options (hierarchical) ───────────────────────────────────
interface CategoryFilterOption {
  id: number
  label: string
  indented: boolean
}

const categoryFilterOptions = computed<CategoryFilterOption[]>(() => {
  const options: CategoryFilterOption[] = []
  for (const parent of categoryStore.topLevel) {
    options.push({ id: parent.id, label: parent.name, indented: false })
    for (const child of categoryStore.childrenOf(parent.id)) {
      options.push({ id: child.id, label: child.name, indented: true })
    }
  }
  // Include orphaned sub-categories (parent not top-level, e.g. edge case)
  const includedIds = new Set(options.map((o) => o.id))
  for (const cat of categoryStore.data) {
    if (!includedIds.has(cat.id)) {
      options.push({ id: cat.id, label: categoryStore.displayPath(cat.id), indented: false })
    }
  }
  return options
})

function onCategoryChange(val: string) {
  filterCategoryId.value = val === '' ? null : Number(val)
}

function onAccountChange(val: string) {
  filterAccountId.value = val === '' ? null : Number(val)
}

function onTypeChange(val: string) {
  filterType.value = val === '' ? null : val
}

// ─── Active filter chips ──────────────────────────────────────────────────────
interface FilterChip {
  key: string
  label: string
  colorCode?: string | null
}

const activeChips = computed<FilterChip[]>(() => {
  const chips: FilterChip[] = []
  if (filterAccountId.value !== null) {
    const acc = accountStore.data.find((a) => a.id === filterAccountId.value)
    if (acc) chips.push({ key: 'account', label: acc.name })
  }
  if (filterCategoryId.value !== null) {
    const cat = categoryStore.byId.get(filterCategoryId.value)
    if (cat) {
      chips.push({
        key: 'category',
        label: categoryStore.displayPath(filterCategoryId.value),
        colorCode: cat.colorCode,
      })
    }
  }
  if (filterType.value) {
    chips.push({ key: 'type', label: capitalize(filterType.value) })
  }
  if (hasAutoSetMonth.value && filterMonth.value) {
    chips.push({ key: 'month', label: formatMonthLabel(filterMonth.value) })
  }
  return chips
})

function removeChip(key: string) {
  if (key === 'account') filterAccountId.value = null
  if (key === 'category') filterCategoryId.value = null
  if (key === 'type') filterType.value = null
  if (key === 'month') {
    filterMonth.value = ''
    hasAutoSetMonth.value = true
  }
}

function clearAll() {
  searchQuery.value = ''
  filterAccountId.value = null
  filterCategoryId.value = null
  filterType.value = null
  filterMonth.value = ''
  hasAutoSetMonth.value = true
}

const hasActiveFilters = computed(
  () =>
    !!searchQuery.value ||
    filterAccountId.value !== null ||
    filterCategoryId.value !== null ||
    !!filterType.value ||
    (hasAutoSetMonth.value && !!filterMonth.value),
)

// ─── Filtered & grouped transactions ─────────────────────────────────────────
const filtered = computed<Transaction[]>(() => {
  const q = searchQuery.value.trim().toLowerCase()
  const month = hasAutoSetMonth.value ? filterMonth.value : effectiveMonth.value

  // Resolve the set of category ids that pass the category filter.
  // Selecting a parent matches all its descendants; selecting a sub matches only that sub.
  let catIds: Set<number> | null = null
  if (filterCategoryId.value !== null) {
    catIds = new Set(categoryStore.descendantIds(filterCategoryId.value))
  }

  return transactionStore.data.filter((t) => {
    // Search: note or category name
    if (q) {
      const note = (t.note ?? '').toLowerCase()
      const cat = categoryStore.byId.get(t.categoryId ?? -1)?.name.toLowerCase() ?? ''
      if (!note.includes(q) && !cat.includes(q)) return false
    }
    // Account
    if (filterAccountId.value !== null && t.accountId !== filterAccountId.value) return false
    // Category (hierarchy-aware)
    if (catIds !== null) {
      if (t.categoryId === null || !catIds.has(t.categoryId)) return false
    }
    // Type
    if (filterType.value && t.type !== filterType.value) return false
    // Month
    if (month && !t.date.startsWith(month)) return false
    return true
  })
})

interface DayGroup {
  date: string
  label: string
  total: number
  transactions: Transaction[]
}

const groupedByDay = computed<DayGroup[]>(() => {
  const map = new Map<string, Transaction[]>()
  for (const t of filtered.value) {
    const list = map.get(t.date) ?? []
    list.push(t)
    map.set(t.date, list)
  }
  // Sort dates descending
  const dates = [...map.keys()].sort().reverse()
  return dates.map((date) => {
    const txns = map.get(date)!
    // Net for the day: income adds, expense subtracts, transfers are internal (net 0).
    const total = txns.reduce((sum, t) => {
      if (t.type === 'income') return sum + t.amount
      if (t.type === 'expense') return sum - t.amount
      return sum
    }, 0)
    return {
      date,
      label: formatDayHeaderWithToday(date),
      total,
      transactions: txns,
    }
  })
})

// Day header: "Today · Tue, Jun 24" / "Yesterday · Mon, Jun 23" / "Sat, Jun 21"
const shortDateFmt = new Intl.DateTimeFormat('en-US', {
  weekday: 'short',
  month: 'short',
  day: 'numeric',
})

function formatDayHeaderWithToday(date: string): string {
  const base = formatDayHeader(date)
  if (base === 'Today' || base === 'Yesterday') {
    return `${base} · ${shortDateFmt.format(new Date(date + 'T00:00:00'))}`
  }
  return base
}

// Subtitle in header: count + date context
const subtitleText = computed(() => {
  const total = transactionStore.data.length
  const shown = filtered.value.length
  const month = hasAutoSetMonth.value ? filterMonth.value : effectiveMonth.value

  if (hasActiveFilters.value && shown !== total) {
    return `${shown} of ${total} match your filters`
  }
  if (month) {
    return `${shown} transactions · ${formatMonthLabel(month)}`
  }
  return `${shown} transaction${shown !== 1 ? 's' : ''}`
})

// ─── No-results state ─────────────────────────────────────────────────────────
const noResults = computed(() => filtered.value.length === 0 && transactionStore.data.length > 0)
const emptyLedger = computed(
  () => filtered.value.length === 0 && transactionStore.data.length === 0,
)
const noResultsQuery = computed(() => searchQuery.value || 'your filters')

// ─── Day total sign color ─────────────────────────────────────────────────────
function dayTotalClass(total: number) {
  if (total > 0) return 'text-green'
  return 'text-secondary'
}

// ─── Edit ──────────────────────────────────────────────────────────────────────
const editingTransaction = ref<Transaction | null>(null)

function openEdit(t: Transaction) {
  editingTransaction.value = t
}

async function handleSave(updated: Transaction) {
  await transactionStore.update(updated)
  editingTransaction.value = null
}

// ─── Delete ──────────────────────────────────────────────────────────────────
const deletingTransaction = ref<Transaction | null>(null)

function openDelete(t: Transaction) {
  deletingTransaction.value = t
}

async function handleConfirmDelete() {
  if (!deletingTransaction.value) return
  await transactionStore.remove(deletingTransaction.value.id)
  deletingTransaction.value = null
}

// ─── Helpers ──────────────────────────────────────────────────────────────────
function capitalize(s: string) {
  return s.charAt(0).toUpperCase() + s.slice(1)
}

function formatMonthLabel(ym: string): string {
  if (!ym) return ''
  const [y, m] = ym.split('-').map(Number)
  return new Intl.DateTimeFormat('en-US', { month: 'long', year: 'numeric' }).format(
    new Date(y, m - 1),
  )
}
</script>

<template>
  <section class="flex flex-col h-full px-7 py-6">
    <!-- ── Header ──────────────────────────────────────────────────────────── -->
    <div class="flex items-center justify-between mb-[18px]">
      <div>
        <h1 class="text-h1 text-primary">Transactions</h1>
        <p class="text-[13px] text-secondary mt-[3px]">{{ subtitleText }}</p>
      </div>
    </div>

    <!-- ── Search + filters bar ───────────────────────────────────────────── -->
    <div class="flex items-center gap-2.5 mb-[18px]">
      <!-- Search input -->
      <div
        class="flex flex-1 items-center gap-2 h-10 rounded-control border bg-card px-3 transition-shadow"
        :class="searchQuery ? 'border-accent ring-2 ring-accent/20' : 'border-border'"
      >
        <!-- Search icon -->
        <svg
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
          class="flex-none text-secondary"
        >
          <circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-width="1.75" />
          <path
            d="M9.5 9.5L12.5 12.5"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
          />
        </svg>
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Search note or category…"
          class="flex-1 bg-transparent text-[14px] text-primary outline-none placeholder:text-secondary/60"
        />
        <button
          v-if="searchQuery"
          class="text-secondary hover:text-primary transition-colors"
          aria-label="Clear search"
          @click="searchQuery = ''"
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path
              d="M9 3L3 9M3 3l6 6"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>

      <!-- Category filter (hierarchy-aware) -->
      <div class="relative">
        <select
          :value="filterCategoryId ?? ''"
          class="h-10 rounded-control border border-border bg-card pl-3 pr-8 text-[13px] font-medium text-secondary appearance-none cursor-pointer hover:border-secondary/50 transition-colors outline-none"
          @change="onCategoryChange(($event.target as HTMLSelectElement).value)"
        >
          <option value="">All categories</option>
          <option v-for="opt in categoryFilterOptions" :key="opt.id" :value="opt.id">
            {{ opt.indented ? '   › ' : '' }}{{ opt.label }}
          </option>
        </select>
        <span
          class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-secondary text-[10px]"
          >▾</span
        >
      </div>

      <!-- Account filter -->
      <div class="relative">
        <select
          :value="filterAccountId ?? ''"
          class="h-10 rounded-control border border-border bg-card pl-3 pr-8 text-[13px] font-medium text-secondary appearance-none cursor-pointer hover:border-secondary/50 transition-colors outline-none"
          @change="onAccountChange(($event.target as HTMLSelectElement).value)"
        >
          <option value="">All accounts</option>
          <option v-for="a in accountStore.data" :key="a.id" :value="a.id">
            {{ a.name }}
          </option>
        </select>
        <span
          class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-secondary text-[10px]"
          >▾</span
        >
      </div>

      <!-- Type filter -->
      <div class="relative">
        <select
          :value="filterType ?? ''"
          class="h-10 rounded-control border border-border bg-card pl-3 pr-8 text-[13px] font-medium text-secondary appearance-none cursor-pointer hover:border-secondary/50 transition-colors outline-none"
          @change="onTypeChange(($event.target as HTMLSelectElement).value)"
        >
          <option value="">Type</option>
          <option value="expense">Expense</option>
          <option value="income">Income</option>
          <option value="transfer">Transfer</option>
        </select>
        <span
          class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-secondary text-[10px]"
          >▾</span
        >
      </div>

      <!-- Month filter -->
      <div class="relative">
        <select
          :value="effectiveMonth"
          class="h-10 rounded-control border border-border bg-card pl-3 pr-8 text-[13px] font-medium text-secondary appearance-none cursor-pointer hover:border-secondary/50 transition-colors outline-none"
          @change="onMonthChange(($event.target as HTMLSelectElement).value)"
        >
          <option value="">All time</option>
          <option v-for="m in monthOptions" :key="m" :value="m">
            {{ formatMonthLabel(m) }}
          </option>
        </select>
        <span
          class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-secondary text-[10px]"
          >▾</span
        >
      </div>
    </div>

    <!-- ── Active filter chips ──────────────────────────────────────────────── -->
    <div v-if="activeChips.length > 0" class="flex flex-wrap items-center gap-2 mb-[18px]">
      <span class="text-[12px] font-medium text-secondary">Filters:</span>
      <span
        v-for="chip in activeChips"
        :key="chip.key"
        class="inline-flex items-center gap-1.5 rounded-pill border border-accent/30 bg-accent/10 px-2.5 py-1.5 text-[12px] font-semibold text-accent"
      >
        <span
          v-if="chip.colorCode"
          class="w-1.5 h-1.5 rounded-full flex-none"
          :style="{ background: chip.colorCode }"
        />
        {{ chip.label }}
        <button
          class="text-accent/60 hover:text-accent transition-colors leading-none"
          :aria-label="`Remove ${chip.label} filter`"
          @click="removeChip(chip.key)"
        >
          ✕
        </button>
      </span>
      <button
        class="text-[12px] font-medium text-secondary hover:text-primary transition-colors cursor-pointer"
        @click="clearAll"
      >
        Clear all
      </button>
    </div>

    <!-- ── Loading ─────────────────────────────────────────────────────────── -->
    <div
      v-if="transactionStore.isLoading"
      class="flex items-center justify-center flex-1 text-secondary text-body"
    >
      Loading…
    </div>

    <!-- ── Error ───────────────────────────────────────────────────────────── -->
    <div
      v-else-if="transactionStore.error"
      class="rounded-card border border-red/20 bg-red/5 p-6 text-red text-body"
    >
      {{ transactionStore.error }}
    </div>

    <!-- ── Empty ledger (truly no transactions at all) ─────────────────────── -->
    <div
      v-else-if="emptyLedger"
      class="flex flex-col items-center justify-center flex-1 text-center py-16"
    >
      <div class="w-14 h-14 rounded-full bg-border/40 flex items-center justify-center mb-4">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" class="text-secondary/60">
          <path
            d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </div>
      <h2 class="text-h2 text-primary mb-2">No transactions yet</h2>
      <p class="text-body text-secondary max-w-xs">
        Add your first transaction using the + button to start tracking your budget.
      </p>
    </div>

    <!-- ── No results (filters matched nothing) — artboard 14 ─────────────── -->
    <div
      v-else-if="noResults"
      class="rounded-card border border-border bg-card shadow-card p-[60px_40px] flex flex-col items-center text-center"
    >
      <!-- Search icon -->
      <div class="w-[60px] h-[60px] rounded-full bg-bg flex items-center justify-center mb-[18px]">
        <svg width="22" height="22" viewBox="0 0 22 22" fill="none" class="text-secondary/50">
          <circle cx="9.5" cy="9.5" r="6.5" stroke="currentColor" stroke-width="2" />
          <path
            d="M14.5 14.5L19.5 19.5"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </div>
      <h2 class="text-[16px] font-semibold text-primary mb-1.5 leading-snug">
        No transactions match "{{ noResultsQuery }}"
      </h2>
      <p class="text-body text-secondary max-w-[340px] mb-5">
        Try a different search term, or widen your filters to see more results.
      </p>
      <button
        class="rounded-control bg-accent/10 px-4 py-2.5 text-[13px] font-semibold text-accent hover:bg-accent/20 transition-colors"
        @click="clearAll"
      >
        Clear search &amp; filters
      </button>
    </div>

    <!-- ── Populated list — artboard 13 ───────────────────────────────────── -->
    <div v-else class="rounded-card border border-border bg-card shadow-card overflow-hidden">
      <template v-for="group in groupedByDay" :key="group.date">
        <!-- Day header -->
        <div
          class="flex items-center justify-between px-4 py-[11px] bg-bg/60 border-b border-border/70"
          :class="{ 'border-t border-t-border/70': groupedByDay.indexOf(group) > 0 }"
        >
          <span class="text-[12px] font-semibold text-secondary">{{ group.label }}</span>
          <span class="num text-[12px] font-semibold" :class="dayTotalClass(group.total)">
            {{ formatSignedCents(group.total) }}
          </span>
        </div>

        <!-- Transaction cards -->
        <TransactionCard
          v-for="t in group.transactions"
          :key="t.id"
          :transaction="t"
          :category="t.categoryId !== null ? categoryStore.byId.get(t.categoryId) : undefined"
          :account="accountStore.data.find((a) => a.id === t.accountId)"
          :to-account="
            t.transferAccountId !== null
              ? accountStore.data.find((a) => a.id === t.transferAccountId)
              : undefined
          "
          @edit="openEdit"
          @delete="openDelete"
        />
      </template>
    </div>

    <!-- ── Edit modal ──────────────────────────────────────────────────────── -->
    <EditTransactionModal
      v-if="editingTransaction"
      :transaction="editingTransaction"
      @save="handleSave"
      @cancel="editingTransaction = null"
    />

    <!-- ── Confirm delete dialog ───────────────────────────────────────────── -->
    <ConfirmDialog
      v-if="deletingTransaction"
      title="Delete transaction?"
      :message="`This will permanently remove &quot;${deletingTransaction.note || 'this transaction'}&quot; and adjust account balances. This cannot be undone.`"
      confirm-label="Delete"
      :danger="true"
      @confirm="handleConfirmDelete"
      @cancel="deletingTransaction = null"
    />
  </section>
</template>
