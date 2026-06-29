<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Transaction } from '../../types'
import { useCategoryStore } from '../../stores/category'
import { useAccountStore } from '../../stores/account'
import { todayISO } from '../../lib/format'

const props = defineProps<{ transaction: Transaction }>()
const emit = defineEmits<{
  save: [t: Transaction]
  cancel: []
}>()

const categoryStore = useCategoryStore()
const accountStore = useAccountStore()

// Local editable copy
const form = ref<Transaction>({ ...props.transaction })

// Amount in display units (dollars) for the input
const amountDisplay = ref(Math.abs(props.transaction.amount) / 100)

// Keep the sign consistent with the type
const signedAmount = computed(() => {
  const abs = Math.round(amountDisplay.value * 100)
  if (form.value.type === 'income') return abs
  if (form.value.type === 'expense') return -abs
  // transfer: positive from the source account's perspective — keep absolute
  return abs
})

watch(
  () => props.transaction,
  (t) => {
    form.value = { ...t }
    amountDisplay.value = Math.abs(t.amount) / 100
  },
)

/**
 * Hierarchical list of categories for the picker.
 * Each entry carries the category id, a display label (with › for subs),
 * and an indent flag so we can style subs.
 */
interface CategoryOption {
  id: number
  label: string
  indented: boolean
}

const hierarchicalCategories = computed<CategoryOption[]>(() => {
  const type = form.value.type
  const options: CategoryOption[] = []

  for (const parent of categoryStore.topLevel) {
    // Skip parents whose type doesn't match (when not a transfer)
    if (type !== 'transfer' && parent.type !== type) continue

    options.push({ id: parent.id, label: parent.name, indented: false })

    for (const child of categoryStore.childrenOf(parent.id)) {
      if (type !== 'transfer' && child.type !== type) continue
      options.push({ id: child.id, label: child.name, indented: true })
    }
  }

  // Also include any sub-categories whose parent is a different type
  // (shouldn't normally happen, but safety net so they're selectable)
  const includedIds = new Set(options.map((o) => o.id))
  for (const cat of categoryStore.data) {
    if (includedIds.has(cat.id)) continue
    if (type !== 'transfer' && cat.type !== type) continue
    options.push({ id: cat.id, label: categoryStore.displayPath(cat.id), indented: false })
  }

  return options
})

function handleSave() {
  emit('save', {
    ...form.value,
    amount: signedAmount.value,
  })
}
</script>

<template>
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-primary/30 backdrop-blur-sm"
    @click.self="emit('cancel')"
  >
    <div
      class="relative w-full max-w-md rounded-card bg-card shadow-pop border border-border"
      role="dialog"
      aria-modal="true"
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-border">
        <h2 class="text-h2 text-primary">Edit Transaction</h2>
        <button
          class="text-secondary hover:text-primary transition-colors"
          aria-label="Close"
          @click="emit('cancel')"
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
      <form class="px-6 pt-5 pb-6 flex flex-col gap-4" @submit.prevent="handleSave">
        <!-- Type -->
        <div class="flex flex-col gap-1.5">
          <label class="text-caption text-secondary">Type</label>
          <div class="flex gap-2">
            <button
              v-for="t in ['expense', 'income', 'transfer'] as const"
              :key="t"
              type="button"
              :class="[
                'flex-1 rounded-control border py-2 text-caption font-semibold capitalize transition-colors',
                form.type === t
                  ? 'border-accent bg-accent/10 text-accent'
                  : 'border-border bg-bg text-secondary hover:bg-bg/80',
              ]"
              @click="form.type = t"
            >
              {{ t }}
            </button>
          </div>
        </div>

        <!-- Amount -->
        <div class="flex flex-col gap-1.5">
          <label for="edit-amount" class="text-caption text-secondary">Amount</label>
          <div
            class="flex items-center gap-2 rounded-control border border-border bg-bg px-3 h-10 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20 transition-shadow"
          >
            <span class="text-secondary text-sm">$</span>
            <input
              id="edit-amount"
              v-model="amountDisplay"
              type="number"
              step="0.01"
              min="0"
              required
              class="flex-1 bg-transparent text-sm text-primary outline-none num"
              placeholder="0.00"
            />
          </div>
        </div>

        <!-- Date -->
        <div class="flex flex-col gap-1.5">
          <label for="edit-date" class="text-caption text-secondary">Date</label>
          <input
            id="edit-date"
            v-model="form.date"
            type="date"
            required
            :max="todayISO()"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          />
        </div>

        <!-- Account -->
        <div class="flex flex-col gap-1.5">
          <label for="edit-account" class="text-caption text-secondary">Account</label>
          <select
            id="edit-account"
            v-model="form.accountId"
            required
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option v-for="a in accountStore.data" :key="a.id" :value="a.id">
              {{ a.name }}
            </option>
          </select>
        </div>

        <!-- Transfer-to account (only for transfers) -->
        <div v-if="form.type === 'transfer'" class="flex flex-col gap-1.5">
          <label for="edit-to-account" class="text-caption text-secondary">To Account</label>
          <select
            id="edit-to-account"
            v-model="form.transferAccountId"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option :value="null">— none —</option>
            <option
              v-for="a in accountStore.data.filter((a) => a.id !== form.accountId)"
              :key="a.id"
              :value="a.id"
            >
              {{ a.name }}
            </option>
          </select>
        </div>

        <!-- Category (not for transfers) — hierarchical picker -->
        <div v-if="form.type !== 'transfer'" class="flex flex-col gap-1.5">
          <label for="edit-category" class="text-caption text-secondary">Category</label>
          <select
            id="edit-category"
            v-model="form.categoryId"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option :value="null">— uncategorized —</option>
            <option
              v-for="opt in hierarchicalCategories"
              :key="opt.id"
              :value="opt.id"
              :class="opt.indented ? 'pl-4' : ''"
            >
              {{ opt.indented ? '   › ' : '' }}{{ opt.label }}
            </option>
          </select>
        </div>

        <!-- Note -->
        <div class="flex flex-col gap-1.5">
          <label for="edit-note" class="text-caption text-secondary">Note</label>
          <input
            id="edit-note"
            v-model="form.note"
            type="text"
            maxlength="200"
            placeholder="Optional note…"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow placeholder:text-secondary/50"
          />
        </div>

        <!-- Actions -->
        <div class="flex justify-end gap-3 pt-1">
          <button
            type="button"
            class="rounded-control border border-border bg-card px-4 py-2 text-caption text-secondary hover:bg-bg transition-colors"
            @click="emit('cancel')"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="rounded-control bg-accent px-4 py-2 text-caption font-semibold text-white hover:bg-accent/90 transition-colors"
          >
            Save changes
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
