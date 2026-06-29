<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import VueApexCharts from 'vue3-apexcharts'
import type ApexCharts from 'apexcharts'
import type { ApexOptions, ApexChartEventOpts } from 'apexcharts'
import type { Transaction, Category } from '../../types'
import { formatCents } from '../../lib/format'
import { useThemeStore } from '../../stores/theme'

const props = defineProps<{
  transactions: Transaction[]
  categoryById: Map<number, Category>
  topLevel: Category[]
  childrenOf: (id: number) => Category[]
  descendantIds: (id: number) => number[]
}>()

const themeStore = useThemeStore()
const isDark = ref(themeStore.resolveDark())

watch(
  () => themeStore.mode,
  () => {
    isDark.value = themeStore.resolveDark()
  },
)

/** Currently drilled-in parent category, or null for top-level view */
const activeDrillId = ref<number | null>(null)

/** Current month expense transactions */
const currentMonthExpenses = computed(() => {
  const now = new Date()
  const curYear = now.getFullYear()
  const curMonth = now.getMonth()
  return props.transactions.filter((tx) => {
    if (tx.type !== 'expense') return false
    const d = new Date(tx.date + 'T00:00:00')
    return d.getFullYear() === curYear && d.getMonth() === curMonth
  })
})

/** Sum of amounts (in cents) for a given set of category IDs */
function sumForIds(ids: Set<number | null>): number {
  return currentMonthExpenses.value
    .filter((tx) => ids.has(tx.categoryId))
    .reduce((acc, tx) => acc + tx.amount, 0)
}

interface Slice {
  label: string
  cents: number
  color: string
  categoryId: number | null
}

/** Top-level view: one slice per top-level category + one uncategorized slice */
const topLevelSlices = computed<Slice[]>(() => {
  const slices: Slice[] = []

  for (const cat of props.topLevel) {
    const ids = new Set(props.descendantIds(cat.id) as (number | null)[])
    const cents = sumForIds(ids)
    if (cents > 0) {
      slices.push({
        label: cat.name,
        cents,
        color: cat.colorCode ?? '#94a3b8',
        categoryId: cat.id,
      })
    }
  }

  // Uncategorized: transactions with null categoryId
  const uncatCents = sumForIds(new Set([null]))
  if (uncatCents > 0) {
    slices.push({
      label: 'Uncategorized',
      cents: uncatCents,
      color: '#94a3b8',
      categoryId: null,
    })
  }

  return slices
})

/** Drill-down view: slices for direct children + direct spend on the parent itself */
const drillSlices = computed<Slice[]>(() => {
  if (activeDrillId.value === null) return []
  const parentId = activeDrillId.value
  const slices: Slice[] = []

  // Direct spend on the parent category itself (not on any child)
  const directCents = sumForIds(new Set([parentId]))
  if (directCents > 0) {
    const parent = props.categoryById.get(parentId)
    slices.push({
      label: 'Direct / Other',
      cents: directCents,
      color: parent?.colorCode ?? '#94a3b8',
      categoryId: parentId,
    })
  }

  // Each direct child
  for (const child of props.childrenOf(parentId)) {
    const cents = sumForIds(new Set([child.id]))
    if (cents > 0) {
      slices.push({
        label: child.name,
        cents,
        color: child.colorCode ?? '#94a3b8',
        categoryId: child.id,
      })
    }
  }

  return slices
})

const activeSlices = computed<Slice[]>(() =>
  activeDrillId.value !== null ? drillSlices.value : topLevelSlices.value,
)

const activeCategoryName = computed<string>(() => {
  if (activeDrillId.value === null) return ''
  return props.categoryById.get(activeDrillId.value)?.name ?? ''
})

const labels = computed(() => activeSlices.value.map((s) => s.label))
const series = computed(() => activeSlices.value.map((s) => Math.round(s.cents / 100)))
const colors = computed(() => activeSlices.value.map((s) => s.color))

const centerLabel = computed(() =>
  activeDrillId.value !== null ? activeCategoryName.value : 'This month',
)

const options = computed<ApexOptions>(() => {
  const dark = isDark.value
  const textColor = dark ? '#94a3b8' : '#64748b'

  return {
    chart: {
      type: 'donut',
      background: 'transparent',
      fontFamily: 'inherit',
      animations: { enabled: true, speed: 400 },
      events: {
        dataPointSelection: (_e: MouseEvent, _chart?: ApexCharts, opts?: ApexChartEventOpts) => {
          if (opts != null) handleSegmentClick(opts.dataPointIndex)
        },
      },
    },
    theme: { mode: dark ? 'dark' : 'light' },
    colors: colors.value,
    labels: labels.value,
    plotOptions: {
      pie: {
        donut: {
          size: '62%',
          labels: {
            show: true,
            total: {
              show: true,
              label: centerLabel.value,
              color: textColor,
              fontSize: '12px',
              formatter: (w: { globals: { seriesTotals: number[] } }) => {
                const total = w.globals.seriesTotals.reduce((a, b) => a + b, 0)
                return formatCents(total * 100)
              },
            },
            value: {
              color: dark ? '#f1f5f9' : '#0f172a',
              fontSize: '14px',
              fontWeight: '600',
              formatter: (val: string) => formatCents(parseFloat(val) * 100),
            },
          },
        },
      },
    },
    stroke: { width: 0 },
    legend: {
      position: 'bottom',
      labels: { colors: textColor },
      fontSize: '12px',
    },
    tooltip: {
      theme: dark ? 'dark' : 'light',
      y: {
        formatter: (val: number) => formatCents(val * 100),
      },
    },
    dataLabels: { enabled: false },
  }
})

const hasData = computed(() => series.value.length > 0 && series.value.some((v) => v > 0))

function handleSegmentClick(dataPointIndex: number): void {
  // Only drill in from the top-level view, and only for real categories (not Uncategorized)
  if (activeDrillId.value !== null) return
  const slice = topLevelSlices.value[dataPointIndex]
  if (!slice || slice.categoryId === null) return
  // Only drill in if the category has children
  const children = props.childrenOf(slice.categoryId)
  if (children.length === 0) return
  activeDrillId.value = slice.categoryId
}

function handleBack(): void {
  activeDrillId.value = null
}
</script>

<template>
  <div v-if="hasData">
    <!-- Drill-down header with Back button -->
    <div v-if="activeDrillId !== null" class="flex items-center gap-2 mb-2 min-h-[28px]">
      <button
        class="flex items-center gap-1 text-[12px] text-accent font-medium hover:opacity-75 transition-opacity"
        @click="handleBack"
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" aria-hidden="true">
          <path
            d="M9 11L5 7l4-4"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        Back
      </button>
      <span class="text-[13px] font-semibold text-primary">{{ activeCategoryName }}</span>
    </div>
    <!-- Spacer to keep height consistent on top-level -->
    <div v-else class="min-h-[28px]" />

    <VueApexCharts type="donut" height="280" :options="options" :series="series" />
  </div>
  <div v-else class="flex flex-col items-center justify-center h-[280px] text-secondary">
    <svg
      width="40"
      height="40"
      viewBox="0 0 40 40"
      fill="none"
      class="mb-3 opacity-30"
      aria-hidden="true"
    >
      <circle cx="20" cy="20" r="18" stroke="currentColor" stroke-width="2.5" />
      <circle cx="20" cy="20" r="10" stroke="currentColor" stroke-width="2.5" />
    </svg>
    <p class="text-[13px]">No expense data for this month</p>
  </div>
</template>
