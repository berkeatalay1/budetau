<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useQuickAddStore } from '../stores/quickAdd'
import { useTransactionStore } from '../stores/transaction'
import { useCategoryStore } from '../stores/category'
import { useAccountStore } from '../stores/account'
import type { QuickAddType } from '../stores/quickAdd'
import type { NewTransaction } from '../types'
import { formatCents, todayISO } from '../lib/format'

// ── Stores ──────────────────────────────────────────────────────────────────
const quickAdd = useQuickAddStore()
const transactionStore = useTransactionStore()
const categoryStore = useCategoryStore()
const accountStore = useAccountStore()

// ── Form state ───────────────────────────────────────────────────────────────
const amountStr = ref('') // raw user input e.g. "12.50"
const categoryQuery = ref('') // text in the category combobox
const selectedCategoryId = ref<number | null>(null)
const date = ref(todayISO())
const note = ref('')
const accountId = ref<number | null>(null)
const transferAccountId = ref<number | null>(null)
const txType = ref<QuickAddType>('expense')
const showDetails = ref(false)

// ── Category autocomplete ────────────────────────────────────────────────────
const autocompleteOpen = ref(false)
const highlightedIndex = ref(0)

/**
 * A flat list of items to render in the dropdown.
 * Each item is either a 'group' header (non-keyboard-navigable, but still
 * selectable because top-level categories are valid targets) or a 'child'
 * entry indented beneath its parent.
 *
 * When the user is filtering (query non-empty):
 *   - Matches against both parent and child names.
 *   - If a child matches, its parent label is shown above it for context.
 *   - If only a parent matches, it appears alone (no indent).
 */
type DropdownItem =
  | { kind: 'parent'; cat: (typeof categoryStore.data)[number] }
  | { kind: 'child'; cat: (typeof categoryStore.data)[number] }

const dropdownItems = computed<DropdownItem[]>(() => {
  const q = categoryQuery.value.trim().toLowerCase()

  if (!q) {
    // Unfiltered: hierarchical view — each top-level followed by its children
    const items: DropdownItem[] = []
    for (const parent of categoryStore.topLevel) {
      items.push({ kind: 'parent', cat: parent })
      for (const child of categoryStore.childrenOf(parent.id)) {
        items.push({ kind: 'child', cat: child })
      }
    }
    return items
  }

  // Filtered: collect matches; include parent header when a child matches
  const items: DropdownItem[] = []
  const addedParentIds = new Set<number>()

  for (const parent of categoryStore.topLevel) {
    const parentMatches = parent.name.toLowerCase().includes(q)
    const children = categoryStore.childrenOf(parent.id)
    const matchingChildren = children.filter((c) => c.name.toLowerCase().includes(q))

    if (parentMatches || matchingChildren.length > 0) {
      if (!addedParentIds.has(parent.id)) {
        items.push({ kind: 'parent', cat: parent })
        addedParentIds.add(parent.id)
      }
      // If parent matched, add all children; else only matching children
      const childrenToShow = parentMatches ? children : matchingChildren
      for (const child of childrenToShow) {
        items.push({ kind: 'child', cat: child })
      }
    }
  }

  // Also include top-level-only categories that were orphaned (no children)
  // — these were already added above when parentMatches, so no duplicates.

  return items
})

const selectedCategory = computed(() =>
  selectedCategoryId.value != null
    ? (categoryStore.data.find((c) => c.id === selectedCategoryId.value) ?? null)
    : null,
)

function openAutocomplete() {
  autocompleteOpen.value = true
  highlightedIndex.value = 0
}

function closeAutocomplete() {
  autocompleteOpen.value = false
}

function pickCategory(id: number) {
  selectedCategoryId.value = id
  categoryQuery.value = categoryStore.displayPath(id)
  closeAutocomplete()
}

function onCategoryInput() {
  selectedCategoryId.value = null // clear selection when user types
  openAutocomplete()
  highlightedIndex.value = 0
}

function onCategoryKeydown(e: KeyboardEvent) {
  if (!autocompleteOpen.value) {
    openAutocomplete()
    return
  }
  const len = dropdownItems.value.length
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    highlightedIndex.value = (highlightedIndex.value + 1) % len
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    highlightedIndex.value = (highlightedIndex.value - 1 + len) % len
  } else if (e.key === 'Enter' && len > 0) {
    e.preventDefault()
    const item = dropdownItems.value[highlightedIndex.value]
    pickCategory(item.cat.id)
  } else if (e.key === 'Escape') {
    closeAutocomplete()
  }
}

// ── Toast ────────────────────────────────────────────────────────────────────
const toastVisible = ref(false)
const toastMessage = ref('')
let toastTimer: ReturnType<typeof setTimeout> | null = null

function showToast(msg: string) {
  toastMessage.value = msg
  toastVisible.value = true
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => {
    toastVisible.value = false
  }, 3500)
}

// ── Derived display helpers ──────────────────────────────────────────────────
const amountCents = computed(() => {
  const v = parseFloat(amountStr.value)
  return isNaN(v) ? 0 : Math.round(v * 100)
})

const submitLabel = computed(() => {
  const suffix =
    txType.value === 'transfer' ? 'transfer' : txType.value === 'income' ? 'income' : 'expense'
  return `Add ${suffix}`
})

const submitBtnClass = computed(() =>
  txType.value === 'income'
    ? 'bg-green text-card hover:bg-green/90'
    : 'bg-accent text-card hover:bg-accent/90',
)

const amountBorderClass = computed(() =>
  txType.value === 'income'
    ? 'border-green ring-2 ring-green/20'
    : 'border-accent ring-2 ring-accent/20',
)

const amountPrefixClass = computed(() =>
  txType.value === 'income' ? 'text-green' : 'text-secondary',
)

const amountPrefix = computed(() => (txType.value === 'income' ? '+$' : '$'))

// ── Validation ───────────────────────────────────────────────────────────────
const errors = ref<string[]>([])

// Account the transaction posts to: explicit selection, else the first account.
const resolvedAccountId = computed<number | null>(
  () => accountId.value ?? accountStore.data[0]?.id ?? null,
)

function validate(): boolean {
  errors.value = []
  if (!amountCents.value || amountCents.value <= 0)
    errors.value.push('Amount is required and must be positive.')
  if (txType.value !== 'transfer' && !selectedCategoryId.value)
    errors.value.push('Category is required.')
  if (txType.value === 'transfer' && !transferAccountId.value)
    errors.value.push('Destination account is required for transfers.')
  if (resolvedAccountId.value == null)
    errors.value.push('No account is available yet — please try again in a moment.')
  return errors.value.length === 0
}

// ── Submit ───────────────────────────────────────────────────────────────────
const isSubmitting = ref(false)

async function submit() {
  if (!validate()) return
  isSubmitting.value = true
  try {
    const payload: NewTransaction = {
      accountId: resolvedAccountId.value as number,
      amount: amountCents.value,
      categoryId: selectedCategoryId.value,
      date: date.value,
      note: note.value.trim() || null,
      type: txType.value,
      transferAccountId: txType.value === 'transfer' ? transferAccountId.value : null,
    }
    await transactionStore.add(payload)
    const catName =
      selectedCategoryId.value != null
        ? categoryStore.displayPath(selectedCategoryId.value)
        : txType.value === 'transfer'
          ? 'transfer'
          : 'transaction'
    showToast(`✓ Added ${formatCents(amountCents.value)} to ${catName}`)
    quickAdd.close()
  } catch (err) {
    errors.value = [err instanceof Error ? err.message : String(err)]
  } finally {
    isSubmitting.value = false
  }
}

// ── Reset on open ─────────────────────────────────────────────────────────────
const amountInputRef = ref<HTMLInputElement | null>(null)

watch(
  () => quickAdd.isOpen,
  async (open) => {
    if (open) {
      txType.value = quickAdd.initialType
      amountStr.value = ''
      categoryQuery.value = ''
      selectedCategoryId.value = null
      date.value = todayISO()
      note.value = ''
      accountId.value = null
      transferAccountId.value = null
      showDetails.value = false
      errors.value = []
      autocompleteOpen.value = false

      // Fetch data if empty (awaited so a fast submit can't beat the load)
      if (categoryStore.data.length === 0) await categoryStore.fetchAll()
      if (accountStore.data.length === 0) await accountStore.fetchAll()

      await nextTick()
      amountInputRef.value?.focus()
    }
  },
)

// ── Keyboard: Esc closes ──────────────────────────────────────────────────────
function handleGlobalKeydown(e: KeyboardEvent) {
  if (!quickAdd.isOpen) return
  if (e.key === 'Escape') {
    e.preventDefault()
    quickAdd.close()
  }
  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
    e.preventDefault()
    submit()
  }
}

onMounted(() => window.addEventListener('keydown', handleGlobalKeydown))
onUnmounted(() => window.removeEventListener('keydown', handleGlobalKeydown))

// ── Dismiss autocomplete on outside click ────────────────────────────────────
function onScrimClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains('modal-scrim')) {
    quickAdd.close()
  }
}

function closeAutocompleteDelayed() {
  setTimeout(closeAutocomplete, 150)
}
</script>

<template>
  <!-- Toast: always mounted so it outlives the modal closure -->
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 translate-y-2"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition duration-200 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 translate-y-2"
    >
      <div
        v-if="toastVisible"
        class="fixed bottom-6 left-1/2 z-[200] -translate-x-1/2 flex items-center gap-3 rounded-[10px] bg-[#1f2937] px-4 py-3 shadow-[0_8px_24px_rgba(16,24,40,.18)]"
        style="color: #fff"
      >
        <div
          class="flex h-5 w-5 flex-none items-center justify-center rounded-full bg-green text-xs font-bold text-card"
        >
          ✓
        </div>
        <span class="text-[13px] font-medium" style="color: #fff">{{ toastMessage }}</span>
      </div>
    </Transition>
  </Teleport>

  <!-- Modal -->
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-150 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-100 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="quickAdd.isOpen"
        class="modal-scrim fixed inset-0 z-[100] flex items-start justify-center overflow-auto bg-[rgba(15,17,22,.42)] px-4 py-14"
        @click="onScrimClick"
        @keydown.stop
      >
        <!-- Panel -->
        <div
          class="w-full max-w-[440px] rounded-[14px] bg-card shadow-[0_24px_64px_rgba(0,0,0,.32)] overflow-hidden"
          @click.stop
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-5 pb-[14px] pt-[18px]">
            <span class="text-[15px] font-semibold text-primary">New transaction</span>
            <button
              type="button"
              class="rounded-md bg-[#f1f3f5] dark:bg-border/30 border border-border px-[7px] py-1 text-[11px] font-semibold text-secondary hover:bg-border/40 transition-colors"
              @click="quickAdd.close()"
            >
              Esc
            </button>
          </div>

          <!-- Body -->
          <div class="flex flex-col gap-4 px-5 pb-2 pt-1">
            <!-- Amount -->
            <div>
              <label class="mb-[7px] block text-[12px] font-semibold text-secondary">Amount</label>
              <div
                class="flex h-[50px] items-center rounded-control border px-[14px] bg-card transition-all"
                :class="amountBorderClass"
              >
                <span class="num text-[24px] font-semibold" :class="amountPrefixClass">{{
                  amountPrefix
                }}</span>
                <input
                  ref="amountInputRef"
                  v-model="amountStr"
                  type="number"
                  min="0"
                  step="0.01"
                  placeholder="0.00"
                  class="num ml-[3px] flex-1 bg-transparent text-[24px] font-semibold text-primary outline-none placeholder:text-secondary/40 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                  @keydown.enter.prevent="submit"
                />
              </div>
            </div>

            <!-- Category + Date row (when details are shown, side by side) -->
            <div :class="showDetails ? 'grid grid-cols-2 gap-3' : 'flex flex-col gap-4'">
              <!-- Category autocomplete -->
              <div class="relative">
                <label class="mb-[7px] block text-[12px] font-semibold text-secondary"
                  >Category</label
                >
                <div
                  class="flex h-[42px] items-center gap-2 rounded-control border border-border bg-card px-3"
                  :class="autocompleteOpen ? 'ring-2 ring-accent/20 border-accent' : ''"
                >
                  <!-- Color dot when selected -->
                  <span
                    v-if="selectedCategory && !autocompleteOpen"
                    class="h-[10px] w-[10px] flex-none rounded-full"
                    :style="{ background: selectedCategory.colorCode ?? '#9ca3af' }"
                  />
                  <input
                    v-model="categoryQuery"
                    type="text"
                    placeholder="Search categories…"
                    class="flex-1 bg-transparent text-[14px] text-primary outline-none placeholder:text-secondary/40"
                    @focus="openAutocomplete"
                    @blur="closeAutocompleteDelayed"
                    @input="onCategoryInput"
                    @keydown="onCategoryKeydown"
                  />
                </div>

                <!-- Hierarchical Dropdown -->
                <div
                  v-if="autocompleteOpen && dropdownItems.length > 0"
                  class="absolute left-0 right-0 top-[72px] z-[110] max-h-[260px] overflow-y-auto rounded-[10px] border border-border bg-card p-[5px] shadow-[0_12px_28px_rgba(16,24,40,.16)]"
                >
                  <template v-for="(item, i) in dropdownItems" :key="`${item.kind}-${item.cat.id}`">
                    <!-- Parent / top-level category row -->
                    <button
                      v-if="item.kind === 'parent'"
                      type="button"
                      class="flex w-full items-center gap-[9px] rounded-[7px] px-[10px] py-[8px] text-left transition-colors"
                      :class="i === highlightedIndex ? 'bg-accent/10' : 'hover:bg-bg'"
                      @mousedown.prevent="pickCategory(item.cat.id)"
                    >
                      <span
                        class="h-[9px] w-[9px] flex-none rounded-full"
                        :style="{ background: item.cat.colorCode ?? '#9ca3af' }"
                      />
                      <span class="text-[13px] font-semibold text-primary">{{
                        item.cat.name
                      }}</span>
                      <span v-if="i === highlightedIndex" class="ml-auto text-[11px] text-secondary"
                        >↵</span
                      >
                    </button>

                    <!-- Child / sub-category row (indented) -->
                    <button
                      v-else
                      type="button"
                      class="flex w-full items-center gap-[9px] rounded-[7px] py-[7px] pl-[26px] pr-[10px] text-left transition-colors"
                      :class="i === highlightedIndex ? 'bg-accent/10' : 'hover:bg-bg'"
                      @mousedown.prevent="pickCategory(item.cat.id)"
                    >
                      <span class="text-[11px] text-secondary/60">›</span>
                      <span
                        class="h-[8px] w-[8px] flex-none rounded-full"
                        :style="{ background: item.cat.colorCode ?? '#9ca3af' }"
                      />
                      <span class="text-[13px] font-medium text-primary">{{ item.cat.name }}</span>
                      <span v-if="i === highlightedIndex" class="ml-auto text-[11px] text-secondary"
                        >↵</span
                      >
                    </button>
                  </template>
                </div>
              </div>

              <!-- Date -->
              <div>
                <label class="mb-[7px] block text-[12px] font-semibold text-secondary">Date</label>
                <div
                  class="flex h-[42px] items-center justify-between rounded-control border border-border bg-card px-3"
                >
                  <input
                    v-model="date"
                    type="date"
                    class="flex-1 bg-transparent text-[14px] text-primary outline-none"
                  />
                </div>
              </div>
            </div>

            <!-- Expanded details section -->
            <template v-if="showDetails">
              <!-- Note -->
              <div>
                <label class="mb-[7px] block text-[12px] font-semibold text-secondary">Note</label>
                <div
                  class="flex h-[42px] items-center rounded-control border border-border bg-card px-3"
                >
                  <input
                    v-model="note"
                    type="text"
                    placeholder="Optional note…"
                    class="flex-1 bg-transparent text-[14px] text-primary outline-none placeholder:text-secondary/40"
                  />
                </div>
              </div>

              <!-- Type segmented control -->
              <div>
                <label class="mb-[7px] block text-[12px] font-semibold text-secondary">Type</label>
                <div
                  class="flex w-full gap-[2px] rounded-[9px] bg-[#f1f3f5] dark:bg-border/20 p-[3px]"
                >
                  <button
                    v-for="opt in ['expense', 'income', 'transfer'] as const"
                    :key="opt"
                    type="button"
                    class="flex-1 rounded-[7px] py-2 text-center text-[13px] font-semibold capitalize transition-all"
                    :class="
                      txType === opt
                        ? opt === 'income'
                          ? 'bg-card text-green shadow-[0_1px_2px_rgba(16,24,40,.1)]'
                          : 'bg-card text-primary shadow-[0_1px_2px_rgba(16,24,40,.1)]'
                        : 'text-secondary'
                    "
                    @click="txType = opt"
                  >
                    {{ opt.charAt(0).toUpperCase() + opt.slice(1) }}
                  </button>
                </div>
              </div>

              <!-- Transfer: From + To accounts -->
              <template v-if="txType === 'transfer'">
                <div class="grid grid-cols-2 gap-3">
                  <div>
                    <label class="mb-[7px] block text-[12px] font-semibold text-secondary"
                      >From account</label
                    >
                    <div
                      class="flex h-[42px] items-center justify-between rounded-control border border-border bg-card px-3"
                    >
                      <select
                        v-model="accountId"
                        class="flex-1 bg-transparent text-[14px] text-primary outline-none appearance-none"
                      >
                        <option :value="null" disabled>Select…</option>
                        <option v-for="acc in accountStore.data" :key="acc.id" :value="acc.id">
                          {{ acc.name }}
                        </option>
                      </select>
                      <span class="ml-2 text-[10px] text-secondary">▾</span>
                    </div>
                  </div>
                  <div>
                    <label class="mb-[7px] block text-[12px] font-semibold text-accent"
                      >To account</label
                    >
                    <div
                      class="flex h-[42px] items-center justify-between rounded-control border border-accent bg-card px-3 ring-2 ring-accent/20"
                    >
                      <select
                        v-model="transferAccountId"
                        class="flex-1 bg-transparent text-[14px] text-primary outline-none appearance-none"
                      >
                        <option :value="null" disabled>Select…</option>
                        <option v-for="acc in accountStore.data" :key="acc.id" :value="acc.id">
                          {{ acc.name }}
                        </option>
                      </select>
                      <span class="ml-2 text-[10px] text-secondary">▾</span>
                    </div>
                  </div>
                </div>
              </template>

              <!-- Non-transfer: single Account -->
              <template v-else>
                <div>
                  <label class="mb-[7px] block text-[12px] font-semibold text-secondary"
                    >Account</label
                  >
                  <div
                    class="flex h-[42px] items-center justify-between rounded-control border border-border bg-card px-3"
                  >
                    <select
                      v-model="accountId"
                      class="flex-1 bg-transparent text-[14px] text-primary outline-none appearance-none"
                    >
                      <option :value="null" disabled>Select…</option>
                      <option v-for="acc in accountStore.data" :key="acc.id" :value="acc.id">
                        {{ acc.name }}
                      </option>
                    </select>
                    <span class="ml-2 text-[10px] text-secondary">▾</span>
                  </div>
                </div>
              </template>
            </template>

            <!-- Add details toggle -->
            <div class="flex items-center gap-[10px] pb-1 pt-2">
              <button
                type="button"
                role="switch"
                :aria-checked="showDetails"
                class="relative h-[22px] w-[38px] flex-none rounded-pill transition-colors duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                :class="showDetails ? 'bg-accent' : 'bg-secondary/30'"
                @click="showDetails = !showDetails"
              >
                <span
                  class="absolute top-[2px] h-[18px] w-[18px] rounded-full bg-card shadow-[0_1px_2px_rgba(0,0,0,.2)] transition-all duration-200"
                  :class="showDetails ? 'left-[18px]' : 'left-[2px]'"
                />
              </button>
              <span class="text-[13px] font-medium text-primary">
                {{ showDetails ? 'Details shown' : 'Add details' }}
                <span v-if="!showDetails" class="font-normal text-secondary">
                  — note, account, type</span
                >
              </span>
            </div>

            <!-- Validation errors -->
            <div v-if="errors.length" class="rounded-control bg-red/10 px-3 py-2">
              <p v-for="err in errors" :key="err" class="text-[12px] text-red">{{ err }}</p>
            </div>
          </div>

          <!-- Footer -->
          <div
            class="mt-[14px] flex items-center justify-end gap-[10px] border-t border-border/60 px-5 py-4"
          >
            <button
              type="button"
              class="rounded-control border border-border bg-card px-[15px] py-[10px] text-[13px] font-semibold text-primary hover:bg-bg transition-colors"
              @click="quickAdd.close()"
            >
              Cancel
            </button>
            <button
              type="button"
              class="inline-flex items-center gap-2 rounded-control px-4 py-[10px] text-[13px] font-semibold transition-colors disabled:opacity-60"
              :class="submitBtnClass"
              :disabled="isSubmitting"
              @click="submit"
            >
              <span>{{ submitLabel }}</span>
              <span v-if="amountCents > 0" class="num">{{ formatCents(amountCents) }}</span>
              <span class="opacity-70">↵</span>
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
