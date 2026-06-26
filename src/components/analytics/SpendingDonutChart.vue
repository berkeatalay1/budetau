<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import VueApexCharts from 'vue3-apexcharts'
import type { ApexOptions } from 'apexcharts'
import type { Transaction, Category } from '../../types'
import { formatCents } from '../../lib/format'
import { useThemeStore } from '../../stores/theme'

const props = defineProps<{
  transactions: Transaction[]
  categoryById: Map<number, Category>
}>()

const themeStore = useThemeStore()
const isDark = ref(themeStore.resolveDark())

watch(
  () => themeStore.mode,
  () => {
    isDark.value = themeStore.resolveDark()
  },
)

/** Current month's expense transactions grouped by category */
const grouped = computed(() => {
  const now = new Date()
  const curYear = now.getFullYear()
  const curMonth = now.getMonth()

  const map = new Map<number | null, number>()

  for (const tx of props.transactions) {
    if (tx.type !== 'expense') continue
    const d = new Date(tx.date + 'T00:00:00')
    if (d.getFullYear() !== curYear || d.getMonth() !== curMonth) continue
    const key = tx.categoryId
    map.set(key, (map.get(key) ?? 0) + tx.amount)
  }

  return map
})

const labels = computed(() => {
  return [...grouped.value.keys()].map((id) =>
    id !== null ? (props.categoryById.get(id)?.name ?? 'Unknown') : 'Uncategorized',
  )
})

const series = computed(() => [...grouped.value.values()].map((cents) => Math.round(cents / 100)))

const colors = computed(() =>
  [...grouped.value.keys()].map((id) => {
    if (id === null) return '#94a3b8'
    const cat = props.categoryById.get(id)
    return cat?.colorCode ?? '#94a3b8'
  }),
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
              label: 'This month',
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
</script>

<template>
  <div v-if="hasData">
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
