<script setup lang="ts">
import { computed, watch, ref } from 'vue'
import VueApexCharts from 'vue3-apexcharts'
import type { ApexOptions } from 'apexcharts'
import type { Transaction } from '../../types'
import { formatCents } from '../../lib/format'
import { useThemeStore } from '../../stores/theme'

const props = defineProps<{
  transactions: Transaction[]
}>()

const themeStore = useThemeStore()

/** Build an array of the last 6 months: [{ year, month (0-indexed), label }] */
function last6Months() {
  const now = new Date()
  return Array.from({ length: 6 }, (_, i) => {
    const d = new Date(now.getFullYear(), now.getMonth() - 5 + i, 1)
    return {
      year: d.getFullYear(),
      month: d.getMonth(),
      label: d.toLocaleString('en-US', { month: 'short', year: '2-digit' }),
    }
  })
}

const months = last6Months()

/** Aggregate transactions into income/expense per month (in dollars, not cents). */
function aggregate(txs: Transaction[]) {
  const income = months.map((m) =>
    txs
      .filter((tx) => {
        if (tx.type !== 'income') return false
        const d = new Date(tx.date + 'T00:00:00')
        return d.getFullYear() === m.year && d.getMonth() === m.month
      })
      .reduce((sum, tx) => sum + tx.amount / 100, 0),
  )
  const expense = months.map((m) =>
    txs
      .filter((tx) => {
        if (tx.type !== 'expense') return false
        const d = new Date(tx.date + 'T00:00:00')
        return d.getFullYear() === m.year && d.getMonth() === m.month
      })
      .reduce((sum, tx) => sum + tx.amount / 100, 0),
  )
  return { income, expense }
}

const series = computed(() => {
  const { income, expense } = aggregate(props.transactions)
  return [
    { name: 'Income', data: income },
    { name: 'Expenses', data: expense },
  ]
})

const isDark = ref(themeStore.resolveDark())

// re-resolve when theme mode changes
watch(
  () => themeStore.mode,
  () => {
    isDark.value = themeStore.resolveDark()
  },
)

const options = computed<ApexOptions>(() => {
  const dark = isDark.value
  const textColor = dark ? '#94a3b8' : '#64748b'
  const gridColor = dark ? '#1e293b' : '#f1f5f9'
  const incomeColor = dark ? '#34d399' : '#059669'
  const expenseColor = dark ? '#f87171' : '#dc2626'

  return {
    chart: {
      type: 'area',
      background: 'transparent',
      toolbar: { show: false },
      fontFamily: 'inherit',
      animations: { enabled: true, speed: 400 },
    },
    theme: { mode: dark ? 'dark' : 'light' },
    colors: [incomeColor, expenseColor],
    fill: {
      type: 'gradient',
      gradient: {
        shadeIntensity: 1,
        opacityFrom: 0.25,
        opacityTo: 0.02,
        stops: [0, 100],
      },
    },
    stroke: { curve: 'smooth', width: 2 },
    xaxis: {
      categories: months.map((m) => m.label),
      axisBorder: { show: false },
      axisTicks: { show: false },
      labels: { style: { colors: textColor, fontSize: '12px' } },
    },
    yaxis: {
      labels: {
        style: { colors: textColor, fontSize: '12px' },
        formatter: (val: number) => formatCents(Math.round(val * 100)),
      },
    },
    grid: {
      borderColor: gridColor,
      strokeDashArray: 4,
      xaxis: { lines: { show: false } },
    },
    tooltip: {
      theme: dark ? 'dark' : 'light',
      y: {
        formatter: (val: number) => formatCents(Math.round(val * 100)),
      },
    },
    legend: {
      labels: { colors: textColor },
      position: 'top',
      horizontalAlign: 'right',
    },
    dataLabels: { enabled: false },
  }
})
</script>

<template>
  <VueApexCharts type="area" height="240" :options="options" :series="series" />
</template>
