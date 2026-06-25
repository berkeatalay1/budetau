<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useBudgetStore } from '../../stores/budget'
import { useCategoryStore } from '../../stores/category'
import type { NewBudget } from '../../types'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

const budgetStore = useBudgetStore()
const categoryStore = useCategoryStore()

const categoryId = ref<number | null>(null)
const amountStr = ref('')
const period = ref('monthly')
const error = ref<string | null>(null)
const isSubmitting = ref(false)
const amountRef = ref<HTMLInputElement | null>(null)

// Expense categories that don't already have a budget.
const availableCategories = computed(() => {
  const budgeted = new Set(budgetStore.data.map((b) => b.categoryId))
  return categoryStore.data.filter((c) => c.type === 'expense' && !budgeted.has(c.id))
})

const amountCents = computed(() => {
  const v = parseFloat(amountStr.value)
  return isNaN(v) ? 0 : Math.round(v * 100)
})

function reset(): void {
  categoryId.value = availableCategories.value[0]?.id ?? null
  amountStr.value = ''
  period.value = 'monthly'
  error.value = null
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      if (categoryStore.data.length === 0) await categoryStore.fetchAll()
      if (budgetStore.data.length === 0) await budgetStore.fetchAll()
      reset()
      await nextTick()
      amountRef.value?.focus()
    }
  },
)

async function submit(): Promise<void> {
  error.value = null
  if (!categoryId.value) {
    error.value = 'Pick a category.'
    return
  }
  if (amountCents.value <= 0) {
    error.value = 'Enter a budget amount.'
    return
  }
  const category = categoryStore.byId.get(categoryId.value)
  if (!category) {
    error.value = 'Category not found.'
    return
  }
  isSubmitting.value = true
  try {
    const payload: NewBudget = {
      profileId: category.profileId,
      categoryId: categoryId.value,
      period: period.value,
      amount: amountCents.value,
    }
    await budgetStore.add(payload)
    emit('close')
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    isSubmitting.value = false
  }
}

function onScrimKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-primary/40 p-4"
      tabindex="-1"
      @click.self="emit('close')"
      @keydown="onScrimKeydown"
    >
      <div class="w-full max-w-[420px] rounded-card bg-card shadow-pop border border-border p-5">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-h2 text-primary">Set a budget</h2>
          <button
            class="rounded-control border border-border px-2 py-1 text-caption text-secondary hover:bg-bg"
            @click="emit('close')"
          >
            Esc
          </button>
        </div>

        <label class="block text-caption text-secondary mb-1.5">Category</label>
        <select
          v-model.number="categoryId"
          class="w-full mb-4 rounded-control border border-border bg-card px-3 py-2.5 text-body text-primary"
        >
          <option v-if="availableCategories.length === 0" :value="null" disabled>
            All expense categories already have budgets
          </option>
          <option v-for="c in availableCategories" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select>

        <label class="block text-caption text-secondary mb-1.5">Monthly limit</label>
        <div
          class="flex items-center mb-4 rounded-control border border-border bg-card px-3 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20"
        >
          <span class="text-secondary">$</span>
          <input
            ref="amountRef"
            v-model="amountStr"
            inputmode="decimal"
            placeholder="0.00"
            class="num flex-1 bg-transparent px-2 py-2.5 text-body text-primary outline-none"
            @keydown.enter="submit"
          />
        </div>

        <p v-if="error" class="text-caption text-red mb-3">{{ error }}</p>

        <div class="flex justify-end gap-2">
          <button
            class="rounded-control border border-border bg-card px-4 py-2.5 text-body font-semibold text-primary hover:bg-bg"
            @click="emit('close')"
          >
            Cancel
          </button>
          <button
            class="rounded-control bg-accent px-4 py-2.5 text-body font-semibold text-card hover:bg-accent/90 disabled:opacity-50"
            :disabled="isSubmitting || availableCategories.length === 0"
            @click="submit"
          >
            Create budget
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
