<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useBudgetStore } from '../../stores/budget'
import { useCategoryStore } from '../../stores/category'
import { formatCents } from '../../lib/format'
import type { Budget } from '../../types'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

const budgetStore = useBudgetStore()
const categoryStore = useCategoryStore()

// ─── editing state ────────────────────────────────────────────────────────────
// Map from budget id → draft edits (amountStr, thresholdStr)
interface DraftRow {
  amountStr: string
  thresholdStr: string
  saving: boolean
  error: string | null
}

const drafts = ref<Map<number, DraftRow>>(new Map())
const confirmDeleteId = ref<number | null>(null)
const isDeleting = ref(false)

// ─── enrich budgets with category info ───────────────────────────────────────
interface BudgetItem {
  budget: Budget
  name: string
  colorCode: string | null
}

const items = computed<BudgetItem[]>(() =>
  budgetStore.data.map((b) => {
    const cat = categoryStore.byId.get(b.categoryId)
    return {
      budget: b,
      name: cat?.name ?? 'Unknown',
      colorCode: cat?.colorCode ?? null,
    }
  }),
)

function initDraft(b: Budget): DraftRow {
  return {
    amountStr: (b.amount / 100).toFixed(2),
    thresholdStr: String(Math.round(b.alertThreshold * 100)),
    saving: false,
    error: null,
  }
}

function getDraft(id: number): DraftRow {
  if (!drafts.value.has(id)) {
    const budget = budgetStore.data.find((b) => b.id === id)
    if (budget) drafts.value.set(id, initDraft(budget))
  }
  return drafts.value.get(id)!
}

// Re-init drafts whenever the modal opens or budget data changes
watch(
  () => [props.open, budgetStore.data] as const,
  ([open]) => {
    if (!open) return
    drafts.value = new Map()
    budgetStore.data.forEach((b) => drafts.value.set(b.id, initDraft(b)))
  },
  { immediate: true, deep: true },
)

watch(
  () => props.open,
  async (open) => {
    if (open) {
      if (categoryStore.data.length === 0) await categoryStore.fetchAll()
      if (budgetStore.data.length === 0) await budgetStore.fetchAll()
      confirmDeleteId.value = null
    }
  },
)

// ─── validation helpers ───────────────────────────────────────────────────────
function parseAmountCents(str: string): number | null {
  const v = parseFloat(str)
  if (isNaN(v) || v <= 0) return null
  return Math.round(v * 100)
}

function parseThresholdFraction(str: string): number | null {
  const v = parseFloat(str)
  if (isNaN(v) || v < 1 || v > 100) return null
  return v / 100
}

// ─── save ─────────────────────────────────────────────────────────────────────
async function save(budget: Budget): Promise<void> {
  const draft = getDraft(budget.id)
  draft.error = null

  const amount = parseAmountCents(draft.amountStr)
  if (amount === null) {
    draft.error = 'Enter a valid amount greater than $0.'
    return
  }
  const alertThreshold = parseThresholdFraction(draft.thresholdStr)
  if (alertThreshold === null) {
    draft.error = 'Alert threshold must be between 1 and 100.'
    return
  }

  draft.saving = true
  try {
    await budgetStore.update({ ...budget, amount, alertThreshold })
    // drafts will be re-initialised by the watcher after fetchAll
  } catch (err) {
    draft.error = err instanceof Error ? err.message : String(err)
  } finally {
    draft.saving = false
  }
}

// ─── delete ───────────────────────────────────────────────────────────────────
async function confirmDelete(id: number): Promise<void> {
  isDeleting.value = true
  try {
    await budgetStore.remove(id)
    confirmDeleteId.value = null
  } catch {
    // ignore — store will surface errors if needed
  } finally {
    isDeleting.value = false
  }
}

function onScrimKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') {
    if (confirmDeleteId.value !== null) {
      confirmDeleteId.value = null
    } else {
      emit('close')
    }
  }
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
      <div
        class="w-full max-w-[520px] rounded-card bg-card shadow-pop border border-border flex flex-col max-h-[80vh]"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-5 pt-5 pb-4 border-b border-border">
          <h2 class="text-h2 text-primary">Manage budgets</h2>
          <button
            class="rounded-control border border-border px-2 py-1 text-caption text-secondary hover:bg-bg"
            @click="emit('close')"
          >
            Esc
          </button>
        </div>

        <!-- Budget list (scrollable) -->
        <div class="flex-1 overflow-y-auto px-5 py-4">
          <!-- Empty state -->
          <div v-if="items.length === 0" class="text-center py-10">
            <p class="text-body text-secondary">No budgets yet.</p>
          </div>

          <div v-else class="flex flex-col gap-4">
            <div
              v-for="item in items"
              :key="item.budget.id"
              class="rounded-card border border-border bg-bg p-4"
            >
              <!-- Category header -->
              <div class="flex items-center gap-2 mb-3">
                <span
                  class="w-[10px] h-[10px] rounded-full flex-none"
                  :style="
                    item.colorCode
                      ? { background: item.colorCode }
                      : { background: 'rgb(var(--c-secondary))' }
                  "
                ></span>
                <span class="font-semibold text-body text-primary">{{ item.name }}</span>
                <span class="text-caption text-secondary ml-auto">
                  Current: {{ formatCents(item.budget.amount) }}/mo ·
                  {{ Math.round(item.budget.alertThreshold * 100) }}% alert
                </span>
              </div>

              <!-- Edit fields -->
              <div class="grid grid-cols-2 gap-3 mb-3">
                <!-- Monthly limit -->
                <div>
                  <label class="block text-caption text-secondary mb-1">Monthly limit</label>
                  <div
                    class="flex items-center rounded-control border border-border bg-card px-3 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20"
                  >
                    <span class="text-secondary text-body">$</span>
                    <input
                      v-model="getDraft(item.budget.id).amountStr"
                      inputmode="decimal"
                      placeholder="0.00"
                      class="num flex-1 bg-transparent px-2 py-2 text-body text-primary outline-none"
                    />
                  </div>
                </div>

                <!-- Alert threshold -->
                <div>
                  <label class="block text-caption text-secondary mb-1">Alert at %</label>
                  <div
                    class="flex items-center rounded-control border border-border bg-card px-3 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20"
                  >
                    <input
                      v-model="getDraft(item.budget.id).thresholdStr"
                      inputmode="decimal"
                      placeholder="90"
                      class="num flex-1 bg-transparent py-2 text-body text-primary outline-none"
                    />
                    <span class="text-secondary text-body">%</span>
                  </div>
                </div>
              </div>

              <!-- Row error -->
              <p v-if="getDraft(item.budget.id).error" class="text-caption text-red mb-2">
                {{ getDraft(item.budget.id).error }}
              </p>

              <!-- Actions -->
              <div class="flex items-center justify-between gap-2">
                <!-- Delete confirm inline -->
                <template v-if="confirmDeleteId === item.budget.id">
                  <span class="text-caption text-secondary">Delete this budget?</span>
                  <div class="flex gap-2">
                    <button
                      class="rounded-control border border-border bg-card px-3 py-1.5 text-caption font-semibold text-primary hover:bg-bg"
                      :disabled="isDeleting"
                      @click="confirmDeleteId = null"
                    >
                      Cancel
                    </button>
                    <button
                      class="rounded-control bg-red/90 px-3 py-1.5 text-caption font-semibold text-card hover:bg-red disabled:opacity-50"
                      :disabled="isDeleting"
                      @click="confirmDelete(item.budget.id)"
                    >
                      {{ isDeleting ? 'Deleting…' : 'Delete' }}
                    </button>
                  </div>
                </template>

                <template v-else>
                  <button
                    class="text-caption font-semibold text-red/80 hover:text-red hover:underline"
                    @click="confirmDeleteId = item.budget.id"
                  >
                    Delete
                  </button>
                  <button
                    class="rounded-control bg-accent px-4 py-1.5 text-caption font-semibold text-card hover:bg-accent/90 disabled:opacity-50"
                    :disabled="getDraft(item.budget.id).saving"
                    @click="save(item.budget)"
                  >
                    {{ getDraft(item.budget.id).saving ? 'Saving…' : 'Save' }}
                  </button>
                </template>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-5 py-4 border-t border-border flex justify-end">
          <button
            class="rounded-control border border-border bg-card px-4 py-2.5 text-body font-semibold text-primary hover:bg-bg"
            @click="emit('close')"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
