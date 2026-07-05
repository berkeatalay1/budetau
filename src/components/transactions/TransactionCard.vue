<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Transaction, Category, Account } from '../../types'
import { formatCents, formatSignedCents } from '../../lib/format'
import { useCategoryStore } from '../../stores/category'

const props = defineProps<{
  transaction: Transaction
  category: Category | undefined
  account: Account | undefined
  toAccount: Account | undefined
}>()

const emit = defineEmits<{
  edit: [t: Transaction]
  delete: [t: Transaction]
}>()

const categoryStore = useCategoryStore()

const isHovered = ref(false)

/**
 * Category label for display purposes only. Income is never budgeted to a
 * category at entry time — it's an inflow to Ready to Assign (see
 * docs/HANDOFF.html §7) — so a null-category income row reads "Ready to
 * Assign" instead of the generic "Uncategorized" that categoryStore.displayPath
 * returns for other null-category rows (e.g. an uncategorized expense).
 * This is copy-only: it does not touch displayPath or any store/data logic.
 */
function categoryLabel(): string {
  if (props.transaction.categoryId == null && props.transaction.type === 'income') {
    return 'Ready to Assign'
  }
  return categoryStore.displayPath(props.transaction.categoryId)
}

// Subtitle line: "Parent › Sub · Account" for sub-categories, "Category · Account" for top-level, or transfer info
const subtitle = computed(() => {
  if (props.transaction.type === 'transfer') {
    const from = props.account?.name ?? '?'
    const to = props.toAccount?.name ?? '?'
    return `Transfer · ${from} → ${to}`
  }
  const acc = props.account?.name ?? '?'
  return `${categoryLabel()} · ${acc}`
})

// Display label for the category icon: first letter of category name, or ⇄ for transfers
const iconLabel = computed(() => {
  if (props.transaction.type === 'transfer') return '⇄'
  return (props.category?.name?.[0] ?? '?').toUpperCase()
})

// Icon background: sub-category's own colorCode (data-driven, inline style), neutral for transfers
const iconColor = computed(() => {
  if (props.transaction.type === 'transfer') return null
  return props.category?.colorCode ?? null
})

// Amount display. `amount` is a positive magnitude; the sign comes from type:
// expense shows negative, income positive, transfer is neutral (no sign).
const amountStr = computed(() => {
  const t = props.transaction
  if (t.type === 'expense') return formatSignedCents(-t.amount)
  if (t.type === 'income') return formatSignedCents(t.amount)
  return formatCents(t.amount)
})

const amountClass = computed(() => {
  if (props.transaction.type === 'income') return 'text-green'
  if (props.transaction.type === 'transfer') return 'text-secondary'
  return 'text-primary'
})

// Note or fall back to category name
const title = computed(() => props.transaction.note || props.category?.name || 'Transaction')
</script>

<template>
  <div
    class="relative flex items-center gap-3.5 px-4 py-3.5 border-b border-border/60 last:border-b-0 transition-colors"
    :class="isHovered ? 'bg-bg' : 'bg-card'"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <!-- Category icon -->
    <div
      class="w-9 h-9 rounded-[10px] flex-none flex items-center justify-center font-semibold text-sm leading-none"
      :class="iconColor ? 'text-white' : 'bg-border text-secondary'"
      :style="iconColor ? { background: iconColor } : {}"
    >
      {{ iconLabel }}
    </div>

    <!-- Text info -->
    <div class="flex-1 min-w-0">
      <div class="text-sm font-semibold text-primary leading-snug truncate">{{ title }}</div>
      <div class="text-xs text-secondary mt-0.5 truncate">{{ subtitle }}</div>
    </div>

    <!-- Row actions (hover) -->
    <transition name="fade">
      <div v-if="isHovered" class="flex items-center gap-1.5 flex-none">
        <button
          class="rounded-control border border-border bg-card px-2.5 py-1.5 text-xs font-semibold text-accent hover:bg-bg transition-colors"
          @click.stop="emit('edit', transaction)"
        >
          Edit
        </button>
        <button
          class="rounded-control border border-red/40 bg-card px-2.5 py-1.5 text-xs font-semibold text-red hover:bg-red/5 transition-colors"
          @click.stop="emit('delete', transaction)"
        >
          Delete
        </button>
      </div>
    </transition>

    <!-- Amount -->
    <span
      class="num text-[15px] font-semibold min-w-[72px] text-right flex-none"
      :class="amountClass"
    >
      {{ amountStr }}
    </span>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.12s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
