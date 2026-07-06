<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useAccountStore } from '../../stores/account'
import { useCategoryStore } from '../../stores/category'
import type { NewRecurringRule, RecurringRule } from '../../types'
import { todayISO } from '../../lib/format'

const props = defineProps<{
  rule?: RecurringRule | null
  profileId: number
}>()

const emit = defineEmits<{
  save: [payload: NewRecurringRule | RecurringRule]
  cancel: []
}>()

const accountStore = useAccountStore()
const categoryStore = useCategoryStore()

// ── Form state ───────────────────────────────────────────────────────────────
type RuleType = 'expense' | 'income' | 'transfer'

const type = ref<RuleType>('expense')
const amountStr = ref('')
const accountId = ref<number | null>(null)
const categoryId = ref<number | null>(null)
const transferAccountId = ref<number | null>(null)
const cadence = ref<'weekly' | 'biweekly' | 'monthly' | 'yearly'>('monthly')
const nextDue = ref(todayISO())
const note = ref('')
const active = ref(true)

const isEdit = computed(() => props.rule != null)

// ── Validation errors — declared before resetForm() since the immediate
// watch below calls resetForm() synchronously during setup. ─────────────────
const errors = ref<string[]>([])

function resetForm() {
  const r = props.rule
  if (r) {
    type.value = r.type as RuleType
    amountStr.value = (r.amount / 100).toFixed(2)
    accountId.value = r.accountId
    categoryId.value = r.categoryId
    transferAccountId.value = r.transferAccountId
    cadence.value = r.cadence as typeof cadence.value
    nextDue.value = r.nextDue
    note.value = r.note ?? ''
    active.value = r.active
  } else {
    type.value = 'expense'
    amountStr.value = ''
    accountId.value = accountStore.data[0]?.id ?? null
    categoryId.value = null
    transferAccountId.value = null
    cadence.value = 'monthly'
    nextDue.value = todayISO()
    note.value = ''
    active.value = true
  }
  errors.value = []
}

watch(() => props.rule, resetForm, { immediate: true })

// Income/transfer never carry a category — clear any stale id when switching
// away from expense, mirroring QuickAddModal's guard (see docs/HANDOFF.html §7).
watch(type, (t) => {
  if (t !== 'expense') categoryId.value = null
  if (t !== 'transfer') transferAccountId.value = null
})

// ── Options ──────────────────────────────────────────────────────────────────
const typeOptions: { value: RuleType; label: string }[] = [
  { value: 'expense', label: 'Expense' },
  { value: 'income', label: 'Income' },
  { value: 'transfer', label: 'Transfer' },
]

const cadenceOptions: { value: string; label: string }[] = [
  { value: 'weekly', label: 'Weekly' },
  { value: 'biweekly', label: 'Every 2 weeks' },
  { value: 'monthly', label: 'Monthly' },
  { value: 'yearly', label: 'Yearly' },
]

const expenseCategories = computed(() => categoryStore.data.filter((c) => c.type === 'expense'))

const transferDestinations = computed(() =>
  accountStore.data.filter((a) => a.id !== accountId.value),
)

const amountCents = computed(() => {
  const v = parseFloat(amountStr.value)
  return isNaN(v) ? 0 : Math.round(v * 100)
})

// ── Validation ───────────────────────────────────────────────────────────────
function validate(): boolean {
  errors.value = []
  if (!amountCents.value || amountCents.value <= 0) {
    errors.value.push('Amount must be greater than zero.')
  }
  if (accountId.value == null) {
    errors.value.push('An account is required.')
  }
  if (type.value === 'expense' && categoryId.value == null) {
    errors.value.push('Category is required for expenses.')
  }
  if (type.value === 'transfer') {
    if (transferAccountId.value == null) {
      errors.value.push('Destination account is required for transfers.')
    } else if (transferAccountId.value === accountId.value) {
      errors.value.push('Destination account must be different from the source account.')
    }
  }
  if (!nextDue.value) {
    errors.value.push('Next due date is required.')
  }
  return errors.value.length === 0
}

// ── Submit ───────────────────────────────────────────────────────────────────
function handleSave() {
  if (!validate()) return

  const base = {
    profileId: props.profileId,
    accountId: accountId.value as number,
    amount: amountCents.value,
    categoryId: type.value === 'expense' ? categoryId.value : null,
    type: type.value,
    transferAccountId: type.value === 'transfer' ? transferAccountId.value : null,
    note: note.value.trim() || null,
    cadence: cadence.value,
    nextDue: nextDue.value,
    active: active.value,
  }

  if (props.rule) {
    emit('save', { ...base, id: props.rule.id })
  } else {
    emit('save', base)
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('cancel')
}
</script>

<template>
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-primary/30 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="recurring-rule-title"
    @click.self="emit('cancel')"
    @keydown="handleKeydown"
  >
    <div
      class="relative w-full max-w-md max-h-[90vh] overflow-y-auto rounded-card bg-card shadow-pop border border-border"
    >
      <!-- Header -->
      <div
        class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-border sticky top-0 bg-card"
      >
        <h2 id="recurring-rule-title" class="text-h2 text-primary">
          {{ isEdit ? 'Edit recurring transaction' : 'Add recurring transaction' }}
        </h2>
        <button
          type="button"
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
              v-for="opt in typeOptions"
              :key="opt.value"
              type="button"
              class="flex-1 rounded-control border py-2 text-caption font-semibold transition-colors"
              :class="
                type === opt.value
                  ? 'border-accent bg-accent/10 text-accent'
                  : 'border-border bg-bg text-secondary hover:bg-bg/80'
              "
              :aria-pressed="type === opt.value"
              @click="type = opt.value"
            >
              {{ opt.label }}
            </button>
          </div>
        </div>

        <!-- Amount -->
        <div class="flex flex-col gap-1.5">
          <label for="rr-amount" class="text-caption text-secondary"
            >Amount <span class="text-red" aria-hidden="true">*</span></label
          >
          <div
            class="flex items-center gap-2 rounded-control border border-border bg-bg px-3 h-10 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20 transition-shadow"
          >
            <span class="text-secondary text-sm" aria-hidden="true">$</span>
            <input
              id="rr-amount"
              v-model="amountStr"
              type="number"
              step="0.01"
              min="0"
              placeholder="0.00"
              class="flex-1 bg-transparent text-sm text-primary outline-none num"
            />
          </div>
        </div>

        <!-- Account -->
        <div class="flex flex-col gap-1.5">
          <label for="rr-account" class="text-caption text-secondary">Account</label>
          <select
            id="rr-account"
            v-model="accountId"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option :value="null" disabled>Select an account…</option>
            <option v-for="acc in accountStore.data" :key="acc.id" :value="acc.id">
              {{ acc.name }}
            </option>
          </select>
        </div>

        <!-- Category (expense only) -->
        <div v-if="type === 'expense'" class="flex flex-col gap-1.5">
          <label for="rr-category" class="text-caption text-secondary">Category</label>
          <select
            id="rr-category"
            v-model="categoryId"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option :value="null" disabled>Select a category…</option>
            <template v-for="parent in categoryStore.topLevel" :key="parent.id">
              <option v-if="parent.type === 'expense'" :value="parent.id">
                {{ categoryStore.displayPath(parent.id) }}
              </option>
              <option
                v-for="child in categoryStore.childrenOf(parent.id)"
                :key="child.id"
                :value="child.id"
              >
                {{ categoryStore.displayPath(child.id) }}
              </option>
            </template>
          </select>
          <p v-if="expenseCategories.length === 0" class="text-[11px] text-secondary">
            No expense categories yet — add one in the Categories card first.
          </p>
        </div>

        <!-- Income: Ready to Assign note -->
        <div v-else-if="type === 'income'" class="flex flex-col gap-1.5">
          <label class="text-caption text-secondary">Goes to</label>
          <div
            class="flex h-10 items-center gap-2 rounded-control border border-green/30 bg-green/10 px-3"
          >
            <span class="h-[9px] w-[9px] flex-none rounded-full bg-green" />
            <span class="text-sm font-semibold text-green">Ready to Assign</span>
          </div>
          <p class="text-[11px] leading-snug text-secondary">
            Income isn't budgeted to a category — assign it on the Plan screen.
          </p>
        </div>

        <!-- Transfer: destination account -->
        <div v-else class="flex flex-col gap-1.5">
          <label for="rr-transfer-account" class="text-caption text-secondary"
            >Destination account <span class="text-red" aria-hidden="true">*</span></label
          >
          <select
            id="rr-transfer-account"
            v-model="transferAccountId"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option :value="null" disabled>Select a destination…</option>
            <option v-for="acc in transferDestinations" :key="acc.id" :value="acc.id">
              {{ acc.name }}
            </option>
          </select>
        </div>

        <!-- Cadence -->
        <div class="flex flex-col gap-1.5">
          <label for="rr-cadence" class="text-caption text-secondary">Repeats</label>
          <select
            id="rr-cadence"
            v-model="cadence"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option v-for="opt in cadenceOptions" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>
        </div>

        <!-- Next due -->
        <div class="flex flex-col gap-1.5">
          <label for="rr-next-due" class="text-caption text-secondary">Starts on</label>
          <input
            id="rr-next-due"
            v-model="nextDue"
            type="date"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          />
          <p class="text-[11px] leading-snug text-secondary">
            The next occurrence posts automatically when you open Budetau on or after this date.
          </p>
        </div>

        <!-- Note -->
        <div class="flex flex-col gap-1.5">
          <label for="rr-note" class="text-caption text-secondary"
            >Note <span class="text-secondary/50">(optional)</span></label
          >
          <input
            id="rr-note"
            v-model="note"
            type="text"
            maxlength="200"
            placeholder="e.g. Paycheck"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow placeholder:text-secondary/50"
          />
        </div>

        <!-- Active toggle -->
        <div class="flex items-center gap-[10px]">
          <button
            type="button"
            role="switch"
            :aria-checked="active"
            class="relative h-[22px] w-[38px] flex-none rounded-pill transition-colors duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
            :class="active ? 'bg-accent' : 'bg-secondary/30'"
            @click="active = !active"
          >
            <span
              class="absolute top-[2px] h-[18px] w-[18px] rounded-full bg-card shadow-[0_1px_2px_rgba(0,0,0,.2)] transition-all duration-200"
              :class="active ? 'left-[18px]' : 'left-[2px]'"
            />
          </button>
          <span class="text-[13px] font-medium text-primary">{{
            active ? 'Active' : 'Paused'
          }}</span>
        </div>

        <!-- Validation errors -->
        <div v-if="errors.length" class="rounded-control bg-red/10 px-3 py-2">
          <p v-for="err in errors" :key="err" class="text-[12px] text-red">{{ err }}</p>
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
            {{ isEdit ? 'Save changes' : 'Add recurring' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
