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
const isDark = ref(themeStore.resolveDark())

watch(
  () => themeStore.mode,
  () => {
    isDark.value = themeStore.resolveDark()
  },
)

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

const netData = computed(() =>
  months.map((m) => {
    let income = 0
    let expense = 0
    for (const tx of props.transactions) {
      if (tx.type !== 'income' && tx.type !== 'expense') continue
      const d = new Date(tx.date + 'T00:00:00')
      if (d.getFullYear() !== m.year || d.getMonth() !== m.month) continue
      if (tx.type === 'income') income += tx.amount / 100
      else expense += tx.amount / 100
    }
    return Math.round((income - expense) * 100) / 100
  }),
)

const series = computed(() => [{ name: 'Net', data: netData.value }])

const barColors = computed(() =>
  netData.value.map((v) => {
    if (v >= 0) return isDark.value ? '#34d399' : '#059669'
    return isDark.value ? '#f87171' : '#dc2626'
  }),
)

const options = computed<ApexOptions>(() => {
  const dark = isDark.value
  const textColor = dark ? '#94a3b8' : '#64748b'
  const gridColor = dark ? '#1e293b' : '#f1f5f9'

  return {
    chart: {
      type: 'bar',
      background: 'transparent',
      toolbar: { show: false },
      fontFamily: 'inherit',
      animations: { enabled: true, speed: 400 },
    },
    theme: { mode: dark ? 'dark' : 'light' },
    colors: barColors.value,
    plotOptions: {
      bar: {
        borderRadius: 4,
        distributed: true,
        columnWidth: '48%',
        dataLabels: { position: 'top' },
      },
    },
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
    legend: { show: false },
    dataLabels: { enabled: false },
  }
})
</script>

<template>
  <VueApexCharts type="bar" height="240" :options="options" :series="series" />
</template>
