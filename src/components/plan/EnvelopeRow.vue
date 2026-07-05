<script setup lang="ts">
import { ref, watch } from 'vue'
import { formatCents, formatSignedCents } from '../../lib/format'

interface RowValues {
  assigned: number
  activity: number
  available: number
}

const props = defineProps<{
  categoryId: number
  name: string
  row: RowValues
  /** The category's own assigned value (cents) — what the input edits.
   * For a parent row this may differ from `row.assigned` (which is the
   * group roll-up total). */
  editableAssigned: number
  isParent?: boolean
  indent?: boolean
}>()

const emit = defineEmits<{
  commitAssign: [categoryId: number, rawValue: string]
  moveMoney: [categoryId: number]
}>()

// Local draft so the input isn't fighting the store while the user types;
// re-synced whenever the underlying value changes (refetch after commit,
// or a month switch).
const draft = ref(dollarsString(props.editableAssigned))

watch(
  () => props.editableAssigned,
  (val) => {
    draft.value = dollarsString(val)
  },
)

function dollarsString(cents: number): string {
  return (cents / 100).toFixed(2)
}

function handleCommit() {
  emit('commitAssign', props.categoryId, draft.value)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
}

function handleMoveMoney() {
  emit('moveMoney', props.categoryId)
}
</script>

<template>
  <div
    class="group grid grid-cols-[minmax(160px,1fr)_110px_110px_110px_36px] items-center gap-3 border-b border-border/60 px-5 py-2.5 last:border-b-0"
  >
    <!-- Category name -->
    <span
      class="truncate text-[13px]"
      :class="[
        indent ? 'pl-5 text-secondary' : 'font-semibold text-primary',
        isParent ? 'font-semibold text-primary' : '',
      ]"
    >
      {{ name }}
    </span>

    <!-- Assigned (editable) -->
    <div class="flex items-center justify-end gap-1 text-[13px]">
      <span class="text-secondary" aria-hidden="true">$</span>
      <input
        v-model="draft"
        type="number"
        step="0.01"
        class="num w-full rounded-[6px] border border-transparent bg-transparent px-1.5 py-1 text-right text-primary outline-none transition-colors hover:border-border focus:border-accent focus:bg-bg focus:ring-2 focus:ring-accent/20"
        :aria-label="`Assigned for ${name}`"
        @blur="handleCommit"
        @keydown="handleKeydown"
      />
    </div>

    <!-- Activity -->
    <span class="num text-right text-[13px] text-secondary">
      {{ formatSignedCents(row.activity) }}
    </span>

    <!-- Available -->
    <span
      class="num text-right text-[13px] font-semibold"
      :class="row.available < 0 ? 'text-red' : 'text-primary'"
    >
      {{ formatCents(row.available) }}
    </span>

    <!-- Row action -->
    <div
      class="flex justify-end opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
    >
      <button
        type="button"
        class="flex h-6 w-6 items-center justify-center rounded-[6px] text-secondary transition-colors hover:bg-bg hover:text-accent"
        :aria-label="`Move money for ${name}`"
        title="Move money"
        @click="handleMoveMoney"
      >
        <svg width="13" height="13" viewBox="0 0 13 13" fill="none" aria-hidden="true">
          <path
            d="M1 6.5h11M8 3l3.5 3.5L8 10"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
    </div>
  </div>
</template>
