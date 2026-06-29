<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { Category, NewCategory } from '../../types'

// Props: pass `category` to edit an existing one; omit for "add" mode.
const props = defineProps<{
  category?: Category
  topLevelCategories: Category[]
  profileId: number
}>()

const emit = defineEmits<{
  save: [payload: NewCategory | Category]
  cancel: []
}>()

// ── Preset color swatches ─────────────────────────────────────────────────────
const SWATCHES = [
  '#2563eb', // blue / accent
  '#059669', // green
  '#d97706', // amber
  '#dc2626', // red
  '#7c3aed', // violet
  '#db2777', // pink
  '#0891b2', // cyan
  '#65a30d', // lime
  '#ea580c', // orange
  '#6b7280', // gray
]

// ── Form state ────────────────────────────────────────────────────────────────
const name = ref('')
const colorCode = ref<string>(SWATCHES[0])
const type = ref<'expense' | 'income'>('expense')
const parentId = ref<number | null>(null)
const nameError = ref('')

const isEditing = computed(() => props.category !== undefined)

// Pre-fill when editing
function syncFromProp() {
  if (props.category) {
    name.value = props.category.name
    colorCode.value = props.category.colorCode ?? SWATCHES[0]
    type.value = (props.category.type as 'expense' | 'income') ?? 'expense'
    parentId.value = props.category.parentId ?? null
  } else {
    name.value = ''
    colorCode.value = SWATCHES[0]
    type.value = 'expense'
    parentId.value = null
  }
  nameError.value = ''
}

syncFromProp()

watch(() => props.category, syncFromProp)

// ── Available parents: top-level cats of same type only ───────────────────────
const availableParents = computed(() =>
  props.topLevelCategories.filter((c) => {
    if (c.type !== type.value) return false
    // Can't be its own parent
    if (isEditing.value && props.category && c.id === props.category.id) return false
    return true
  }),
)

// When type changes, clear parentId if the selected parent doesn't match
watch(type, () => {
  const stillValid = availableParents.value.some((c) => c.id === parentId.value)
  if (!stillValid) {
    parentId.value = null
  }
})

// ── Save ──────────────────────────────────────────────────────────────────────
function handleSave() {
  nameError.value = ''
  if (!name.value.trim()) {
    nameError.value = 'Name is required.'
    return
  }
  if (isEditing.value && props.category) {
    const updated: Category = {
      ...props.category,
      name: name.value.trim(),
      colorCode: colorCode.value,
      type: type.value,
      parentId: parentId.value,
    }
    emit('save', updated)
  } else {
    const payload: NewCategory = {
      profileId: props.profileId,
      name: name.value.trim(),
      colorCode: colorCode.value,
      type: type.value,
      parentId: parentId.value,
    }
    emit('save', payload)
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
    aria-labelledby="cat-form-title"
    @click.self="emit('cancel')"
    @keydown="handleKeydown"
  >
    <div class="relative w-full max-w-sm rounded-card bg-card shadow-pop border border-border">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-border">
        <h2 id="cat-form-title" class="text-h2 text-primary">
          {{ isEditing ? 'Edit Category' : 'Add Category' }}
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
        <!-- Name -->
        <div class="flex flex-col gap-1.5">
          <label for="cat-name" class="text-caption text-secondary">
            Name <span class="text-red" aria-hidden="true">*</span>
          </label>
          <input
            id="cat-name"
            v-model="name"
            type="text"
            required
            maxlength="80"
            placeholder="e.g. Dining Out"
            class="rounded-control border bg-bg px-3 h-10 text-sm text-primary outline-none focus:ring-2 focus:ring-accent/20 transition-shadow placeholder:text-secondary/50"
            :class="nameError ? 'border-red focus:border-red' : 'border-border focus:border-accent'"
            aria-required="true"
            :aria-invalid="!!nameError"
            :aria-describedby="nameError ? 'cat-name-error' : undefined"
          />
          <p v-if="nameError" id="cat-name-error" class="text-[11px] text-red">{{ nameError }}</p>
        </div>

        <!-- Color swatches -->
        <div class="flex flex-col gap-1.5">
          <label class="text-caption text-secondary">Color</label>
          <div class="flex flex-wrap gap-2 items-center">
            <button
              v-for="swatch in SWATCHES"
              :key="swatch"
              type="button"
              class="h-7 w-7 rounded-full border-2 transition-transform hover:scale-110 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
              :style="{ backgroundColor: swatch }"
              :class="
                colorCode === swatch
                  ? 'border-primary scale-110 shadow-[0_0_0_2px_white,0_0_0_4px_currentColor]'
                  : 'border-transparent'
              "
              :aria-label="`Color ${swatch}`"
              :aria-pressed="colorCode === swatch"
              @click="colorCode = swatch"
            />
            <!-- Custom color input -->
            <label
              class="relative h-7 w-7 cursor-pointer rounded-full border-2 border-dashed border-border hover:border-secondary transition-colors flex items-center justify-center overflow-hidden"
              title="Custom color"
            >
              <span class="text-[9px] text-secondary leading-none select-none">+</span>
              <input
                v-model="colorCode"
                type="color"
                class="absolute inset-0 opacity-0 cursor-pointer"
                aria-label="Custom color picker"
              />
            </label>
            <!-- Preview dot of current custom color (when not in swatches) -->
            <span
              v-if="!SWATCHES.includes(colorCode)"
              class="h-5 w-5 rounded-full border border-border"
              :style="{ backgroundColor: colorCode }"
              aria-hidden="true"
            />
          </div>
        </div>

        <!-- Type -->
        <div class="flex flex-col gap-1.5">
          <label class="text-caption text-secondary">Type</label>
          <div class="inline-flex gap-[2px] rounded-[10px] bg-[#f1f3f5] dark:bg-border/20 p-[3px]">
            <button
              type="button"
              class="flex-1 rounded-[8px] px-5 py-[9px] text-[13px] font-semibold transition-all"
              :class="
                type === 'expense'
                  ? 'bg-card text-primary shadow-[0_1px_2px_rgba(16,24,40,.1)]'
                  : 'text-secondary hover:text-primary'
              "
              :aria-pressed="type === 'expense'"
              @click="type = 'expense'"
            >
              Expense
            </button>
            <button
              type="button"
              class="flex-1 rounded-[8px] px-5 py-[9px] text-[13px] font-semibold transition-all"
              :class="
                type === 'income'
                  ? 'bg-card text-primary shadow-[0_1px_2px_rgba(16,24,40,.1)]'
                  : 'text-secondary hover:text-primary'
              "
              :aria-pressed="type === 'income'"
              @click="type = 'income'"
            >
              Income
            </button>
          </div>
        </div>

        <!-- Parent (optional) -->
        <div class="flex flex-col gap-1.5">
          <label for="cat-parent" class="text-caption text-secondary">
            Parent category
            <span class="text-secondary/50">(optional — leave empty for top-level)</span>
          </label>
          <select
            id="cat-parent"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
            :value="parentId ?? ''"
            @change="
              parentId =
                ($event.target as HTMLSelectElement).value === ''
                  ? null
                  : Number(($event.target as HTMLSelectElement).value)
            "
          >
            <option value="">No parent (top-level)</option>
            <option v-for="parent in availableParents" :key="parent.id" :value="parent.id">
              {{ parent.name }}
            </option>
          </select>
          <p v-if="availableParents.length === 0" class="text-[11px] text-secondary/70">
            No top-level {{ type }} categories yet — add one first.
          </p>
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
            {{ isEditing ? 'Save changes' : 'Add category' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
