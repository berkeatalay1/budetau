<script setup lang="ts">
import { computed, ref } from 'vue'
import { useBudgetMonthStore } from '../../stores/budgetMonth'
import { useCategoryStore } from '../../stores/category'
import { useToastStore } from '../../stores/toast'
import type { Category } from '../../types'
import { formatCents } from '../../lib/format'

const props = defineProps<{
  categories: Category[]
  defaultFromId: number | null
}>()

const emit = defineEmits<{
  close: []
}>()

const budgetMonthStore = useBudgetMonthStore()
const categoryStore = useCategoryStore()
const toast = useToastStore()

const fromId = ref<number | null>(props.defaultFromId)
const toId = ref<number | null>(null)
const amountDollars = ref('')
const errorMessage = ref('')
const isSaving = ref(false)

const sortedCategories = computed(() =>
  [...props.categories].sort((a, b) => a.name.localeCompare(b.name)),
)

function categoryLabel(category: Category): string {
  return categoryStore.parentOf(category)
    ? `${categoryStore.parentOf(category)!.name} › ${category.name}`
    : category.name
}

function availableFor(categoryId: number | null): number {
  if (categoryId == null) return 0
  return budgetMonthStore.data?.categories.find((c) => c.categoryId === categoryId)?.available ?? 0
}

function assignedFor(categoryId: number): number {
  return budgetMonthStore.data?.categories.find((c) => c.categoryId === categoryId)?.assigned ?? 0
}

function handleClose() {
  emit('close')
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') handleClose()
}

async function handleSubmit() {
  errorMessage.value = ''

  const dollars = Number.parseFloat(amountDollars.value)
  if (Number.isNaN(dollars) || dollars <= 0) {
    errorMessage.value = 'Enter an amount greater than zero.'
    return
  }
  if (fromId.value == null || toId.value == null) {
    errorMessage.value = 'Choose both a source and destination envelope.'
    return
  }
  if (fromId.value === toId.value) {
    errorMessage.value = 'Pick two different envelopes.'
    return
  }

  const cents = Math.round(dollars * 100)
  const fromAssigned = assignedFor(fromId.value)
  const toAssigned = assignedFor(toId.value)

  isSaving.value = true
  try {
    // `assign` is an ABSOLUTE set, not a delta: read current assigned for
    // both envelopes first, then move `cents` from one to the other. Total
    // assigned is unchanged, so Ready to Assign stays the same.
    await budgetMonthStore.assign(fromId.value, fromAssigned - cents)
    await budgetMonthStore.assign(toId.value, toAssigned + cents)
    toast.show('Money moved between envelopes.')
    emit('close')
  } catch (err) {
    errorMessage.value = err instanceof Error ? err.message : String(err)
  } finally {
    isSaving.value = false
  }
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-primary/30 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="move-money-title"
    @click.self="handleClose"
    @keydown="handleKeydown"
  >
    <div class="relative w-full max-w-sm rounded-card border border-border bg-card shadow-pop">
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-border px-6 pb-4 pt-5">
        <h2 id="move-money-title" class="text-h2 text-primary">Move money</h2>
        <button
          type="button"
          class="text-secondary transition-colors hover:text-primary"
          aria-label="Close"
          @click="handleClose"
        >
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none">
            <path
              d="M13.5 4.5L4.5 13.5M4.5 4.5l9 9"
              stroke="currentColor"
              stroke-width="1.75"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>

      <!-- Form -->
      <form class="flex flex-col gap-4 px-6 pb-6 pt-5" @submit.prevent="handleSubmit">
        <!-- From -->
        <div class="flex flex-col gap-1.5">
          <label for="move-from" class="text-caption text-secondary">From envelope</label>
          <select
            id="move-from"
            v-model="fromId"
            class="h-10 rounded-control border border-border bg-bg px-3 text-sm text-primary outline-none transition-shadow focus:border-accent focus:ring-2 focus:ring-accent/20"
          >
            <option :value="null" disabled>Select an envelope</option>
            <option v-for="c in sortedCategories" :key="c.id" :value="c.id">
              {{ categoryLabel(c) }}
            </option>
          </select>
          <p v-if="fromId != null" class="text-[11px] text-secondary">
            Available: {{ formatCents(availableFor(fromId)) }}
          </p>
        </div>

        <!-- To -->
        <div class="flex flex-col gap-1.5">
          <label for="move-to" class="text-caption text-secondary">To envelope</label>
          <select
            id="move-to"
            v-model="toId"
            class="h-10 rounded-control border border-border bg-bg px-3 text-sm text-primary outline-none transition-shadow focus:border-accent focus:ring-2 focus:ring-accent/20"
          >
            <option :value="null" disabled>Select an envelope</option>
            <option v-for="c in sortedCategories" :key="c.id" :value="c.id">
              {{ categoryLabel(c) }}
            </option>
          </select>
          <p v-if="toId != null" class="text-[11px] text-secondary">
            Available: {{ formatCents(availableFor(toId)) }}
          </p>
        </div>

        <!-- Amount -->
        <div class="flex flex-col gap-1.5">
          <label for="move-amount" class="text-caption text-secondary">Amount</label>
          <div
            class="flex h-10 items-center gap-2 rounded-control border border-border bg-bg px-3 transition-shadow focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20"
          >
            <span class="text-sm text-secondary" aria-hidden="true">$</span>
            <input
              id="move-amount"
              v-model="amountDollars"
              type="number"
              step="0.01"
              min="0"
              placeholder="0.00"
              class="num flex-1 bg-transparent text-sm text-primary outline-none"
            />
          </div>
        </div>

        <!-- Error -->
        <p v-if="errorMessage" class="text-[12px] text-red">{{ errorMessage }}</p>

        <!-- Actions -->
        <div class="flex justify-end gap-3 pt-1">
          <button
            type="button"
            class="rounded-control border border-border bg-card px-4 py-2 text-caption text-secondary transition-colors hover:bg-bg"
            @click="handleClose"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="rounded-control bg-accent px-4 py-2 text-caption font-semibold text-white transition-colors hover:bg-accent/90 disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="isSaving"
          >
            {{ isSaving ? 'Moving…' : 'Move money' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
