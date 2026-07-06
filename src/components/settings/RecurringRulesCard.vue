<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useAccountStore } from '../../stores/account'
import { useCategoryStore } from '../../stores/category'
import { useRecurringStore } from '../../stores/recurring'
import { useToastStore } from '../../stores/toast'
import type { NewRecurringRule, RecurringRule } from '../../types'
import { formatCents } from '../../lib/format'
import RecurringRuleModal from './RecurringRuleModal.vue'
import ConfirmDialog from '../transactions/ConfirmDialog.vue'

const accountStore = useAccountStore()
const categoryStore = useCategoryStore()
const recurringStore = useRecurringStore()
const toast = useToastStore()

// ── Modal state ───────────────────────────────────────────────────────────────
const showFormModal = ref(false)
const editingRule = ref<RecurringRule | null>(null)
const deletingRule = ref<RecurringRule | null>(null)

// ── Error banners ─────────────────────────────────────────────────────────────
const actionError = ref<string | null>(null)
const deleteError = ref<string | null>(null)

// ── Profile id — derive from existing data or fall back to 1 ─────────────────
const profileId = computed(() => accountStore.data[0]?.profileId ?? 1)

// ── Fetch on mount if empty ───────────────────────────────────────────────────
onMounted(async () => {
  if (recurringStore.data.length === 0) {
    await recurringStore.fetchAll()
  }
  if (accountStore.data.length === 0) {
    await accountStore.fetchAll()
  }
  if (categoryStore.data.length === 0) {
    await categoryStore.fetchAll()
  }
})

// ── Open add/edit ─────────────────────────────────────────────────────────────
function openAdd() {
  editingRule.value = null
  actionError.value = null
  showFormModal.value = true
}

function openEdit(rule: RecurringRule) {
  editingRule.value = rule
  actionError.value = null
  showFormModal.value = true
}

function closeForm() {
  showFormModal.value = false
  editingRule.value = null
}

// ── Save (add or edit) ────────────────────────────────────────────────────────
async function handleSave(payload: NewRecurringRule | RecurringRule) {
  actionError.value = null
  try {
    if ('id' in payload) {
      await recurringStore.update(payload as RecurringRule)
      toast.show('Recurring transaction updated.')
    } else {
      await recurringStore.add(payload as NewRecurringRule)
      toast.show('Recurring transaction added.')
    }
    closeForm()
  } catch (err) {
    actionError.value = err instanceof Error ? err.message : String(err)
  }
}

// ── Delete ────────────────────────────────────────────────────────────────────
function requestDelete(rule: RecurringRule) {
  deletingRule.value = rule
  deleteError.value = null
}

function cancelDelete() {
  deletingRule.value = null
}

async function confirmDelete() {
  if (!deletingRule.value) return
  deleteError.value = null
  const rule = deletingRule.value
  deletingRule.value = null
  try {
    await recurringStore.remove(rule.id)
    toast.show('Recurring transaction deleted.')
  } catch (err) {
    deleteError.value = err instanceof Error ? err.message : String(err)
  }
}

function dismissActionError() {
  actionError.value = null
}

function dismissDeleteError() {
  deleteError.value = null
}

// ── Display helpers ───────────────────────────────────────────────────────────
const cadenceLabels: Record<string, string> = {
  weekly: 'Weekly',
  biweekly: 'Every 2 weeks',
  monthly: 'Monthly',
  yearly: 'Yearly',
}

function cadenceLabel(cadence: string): string {
  return cadenceLabels[cadence] ?? cadence
}

function ruleLabel(rule: RecurringRule): string {
  if (rule.note && rule.note.trim()) return rule.note.trim()
  if (rule.type === 'income') return 'Ready to Assign'
  if (rule.type === 'transfer') return 'Transfer'
  if (rule.categoryId != null) return categoryStore.displayPath(rule.categoryId)
  return 'Ready to Assign'
}

function accountName(accountId: number): string {
  return accountStore.data.find((a) => a.id === accountId)?.name ?? 'Unknown account'
}

function formatNextDue(iso: string): string {
  const d = new Date(iso + 'T00:00:00')
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  }).format(d)
}

function typeBadgeClass(type: string): string {
  if (type === 'income') return 'bg-green/10 text-green'
  if (type === 'transfer') return 'bg-accent/10 text-accent'
  return 'bg-red/10 text-red'
}

function amountDisplay(rule: RecurringRule): string {
  const prefix = rule.type === 'income' ? '+' : rule.type === 'expense' ? '-' : ''
  return `${prefix}${formatCents(rule.amount)}`
}

function amountClass(rule: RecurringRule): string {
  if (rule.type === 'income') return 'text-green'
  if (rule.type === 'expense') return 'text-primary'
  return 'text-accent'
}
</script>

<template>
  <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
    <!-- Card header -->
    <div class="mb-1 flex items-center justify-between">
      <h2 class="text-[15px] font-semibold text-primary">Recurring transactions</h2>
      <button
        type="button"
        class="flex items-center gap-1.5 rounded-control border border-border bg-bg px-3 py-[7px] text-[12px] font-semibold text-primary transition-colors hover:bg-card"
        @click="openAdd"
      >
        <svg width="11" height="11" viewBox="0 0 11 11" fill="none" aria-hidden="true">
          <path
            d="M5.5 1v9M1 5.5h9"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
          />
        </svg>
        Add recurring
      </button>
    </div>
    <p class="mb-4 text-[13px] leading-relaxed text-secondary">
      Automate paychecks, rent, and other repeating transactions. Due occurrences post automatically
      when you open Budetau.
    </p>

    <!-- Error banners: action (add/edit) and delete -->
    <template
      v-for="[msg, dismiss] in [
        [actionError, dismissActionError],
        [deleteError, dismissDeleteError],
      ] as [string | null, () => void][]"
      :key="msg"
    >
      <div
        v-if="msg"
        class="mb-3 flex items-center gap-2 rounded-[9px] border border-red/40 bg-red/10 px-[13px] py-[10px]"
        role="alert"
      >
        <span class="flex-1 text-[12px] leading-relaxed text-red">{{ msg }}</span>
        <button
          type="button"
          class="text-red/60 hover:text-red transition-colors"
          aria-label="Dismiss error"
          @click="dismiss"
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
    </template>

    <!-- Loading state -->
    <div v-if="recurringStore.isLoading" class="py-4 text-center text-[13px] text-secondary">
      Loading…
    </div>

    <!-- Empty state -->
    <div
      v-else-if="recurringStore.data.length === 0"
      class="rounded-[9px] border border-dashed border-border py-8 text-center"
    >
      <p class="text-[13px] text-secondary">No recurring transactions yet.</p>
      <button
        type="button"
        class="mt-2 text-[13px] font-semibold text-accent hover:underline"
        @click="openAdd"
      >
        Add your first recurring transaction
      </button>
    </div>

    <!-- Rule list -->
    <ul v-else class="flex flex-col divide-y divide-border/60" role="list">
      <li
        v-for="rule in recurringStore.data"
        :key="rule.id"
        class="group flex items-center gap-3 py-[11px] first:pt-0 last:pb-0"
      >
        <!-- Active / paused indicator -->
        <span
          class="h-[9px] w-[9px] flex-none rounded-full"
          :class="rule.active ? 'bg-green' : 'bg-secondary/40'"
          :aria-label="rule.active ? 'Active' : 'Paused'"
          :title="rule.active ? 'Active' : 'Paused'"
        />

        <!-- Label + meta -->
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <p class="truncate text-[13px] font-semibold text-primary leading-snug">
              {{ ruleLabel(rule) }}
            </p>
            <span
              class="inline-flex items-center rounded-pill px-2 py-[2px] text-[10px] font-semibold capitalize"
              :class="typeBadgeClass(rule.type)"
            >
              {{ rule.type }}
            </span>
          </div>
          <p class="text-[11px] text-secondary">
            {{ cadenceLabel(rule.cadence) }} · Next {{ formatNextDue(rule.nextDue) }} ·
            {{ accountName(rule.accountId) }}
          </p>
        </div>

        <!-- Amount -->
        <span class="num text-[13px] font-semibold tabular-nums" :class="amountClass(rule)">
          {{ amountDisplay(rule) }}
        </span>

        <!-- Row actions (visible on hover / focus-within) -->
        <div
          class="ml-2 flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
        >
          <button
            type="button"
            class="rounded-[6px] border border-border bg-card px-2.5 py-1.5 text-[11px] font-semibold text-secondary hover:text-primary transition-colors"
            :aria-label="`Edit ${ruleLabel(rule)}`"
            @click="openEdit(rule)"
          >
            Edit
          </button>
          <button
            type="button"
            class="rounded-[6px] border border-red/30 bg-red/5 px-2.5 py-1.5 text-[11px] font-semibold text-red hover:bg-red/10 transition-colors"
            :aria-label="`Delete ${ruleLabel(rule)}`"
            @click="requestDelete(rule)"
          >
            Delete
          </button>
        </div>
      </li>
    </ul>
  </div>

  <!-- ── Modals ──────────────────────────────────────────────────────────────── -->
  <RecurringRuleModal
    v-if="showFormModal"
    :rule="editingRule"
    :profile-id="profileId"
    @save="handleSave"
    @cancel="closeForm"
  />

  <ConfirmDialog
    v-if="deletingRule"
    title="Delete recurring transaction?"
    :message="`Delete &quot;${ruleLabel(deletingRule)}&quot;? This cannot be undone.`"
    confirm-label="Delete"
    :danger="true"
    @confirm="confirmDelete"
    @cancel="cancelDelete"
  />
</template>
