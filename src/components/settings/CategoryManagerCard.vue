<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useCategoryStore } from '../../stores/category'
import type { Category, NewCategory } from '../../types'
import CategoryFormModal from './CategoryFormModal.vue'
import ConfirmDialog from '../transactions/ConfirmDialog.vue'

const categoryStore = useCategoryStore()

// ── Modal state ───────────────────────────────────────────────────────────────
const showFormModal = ref(false)
const editingCategory = ref<Category | undefined>(undefined)
const deletingCategory = ref<Category | null>(null)

// ── Error banners ─────────────────────────────────────────────────────────────
const actionError = ref<string | null>(null)
const deleteError = ref<string | null>(null)

// ── Profile id — derive from existing data or fall back to 1 ─────────────────
function getProfileId(): number {
  return categoryStore.data[0]?.profileId ?? 1
}

// ── Fetch on mount if empty ───────────────────────────────────────────────────
onMounted(async () => {
  if (categoryStore.data.length === 0) {
    await categoryStore.fetchAll()
  }
})

// ── Open add/edit ─────────────────────────────────────────────────────────────
function openAdd() {
  editingCategory.value = undefined
  actionError.value = null
  showFormModal.value = true
}

function openEdit(cat: Category) {
  editingCategory.value = cat
  actionError.value = null
  showFormModal.value = true
}

function closeForm() {
  showFormModal.value = false
  editingCategory.value = undefined
}

// ── Save (add or edit) ────────────────────────────────────────────────────────
async function handleSave(payload: NewCategory | Category) {
  actionError.value = null
  try {
    if ('id' in payload) {
      await categoryStore.update(payload as Category)
    } else {
      await categoryStore.add(payload as NewCategory)
    }
    closeForm()
  } catch (err) {
    actionError.value = err instanceof Error ? err.message : String(err)
  }
}

// ── Delete ────────────────────────────────────────────────────────────────────
function requestDelete(cat: Category) {
  deletingCategory.value = cat
  deleteError.value = null
}

function cancelDelete() {
  deletingCategory.value = null
}

async function confirmDelete() {
  if (!deletingCategory.value) return
  deleteError.value = null
  const cat = deletingCategory.value
  // Close dialog immediately; surface errors as inline banner
  deletingCategory.value = null
  try {
    await categoryStore.remove(cat.id)
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

// ── Type badge helper ─────────────────────────────────────────────────────────
function typeBadgeClass(type: string): string {
  return type === 'income' ? 'bg-green/10 text-green' : 'bg-red/10 text-red'
}
</script>

<template>
  <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
    <!-- Card header -->
    <div class="mb-1 flex items-center justify-between">
      <h2 class="text-[15px] font-semibold text-primary">Categories</h2>
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
        Add category
      </button>
    </div>
    <p class="mb-4 text-[13px] leading-relaxed text-secondary">
      Organise your spending and income with categories and sub-categories.
    </p>

    <!-- Action error banner (add / edit) -->
    <div
      v-if="actionError"
      class="mb-3 flex items-center gap-2 rounded-[9px] border border-red/40 bg-red/10 px-[13px] py-[10px]"
      role="alert"
    >
      <span class="flex-1 text-[12px] leading-relaxed text-red">{{ actionError }}</span>
      <button
        type="button"
        class="text-red/60 hover:text-red transition-colors"
        aria-label="Dismiss error"
        @click="dismissActionError"
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

    <!-- Delete error banner -->
    <div
      v-if="deleteError"
      class="mb-3 flex items-center gap-2 rounded-[9px] border border-red/40 bg-red/10 px-[13px] py-[10px]"
      role="alert"
    >
      <span class="flex-1 text-[12px] leading-relaxed text-red">{{ deleteError }}</span>
      <button
        type="button"
        class="text-red/60 hover:text-red transition-colors"
        aria-label="Dismiss error"
        @click="dismissDeleteError"
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

    <!-- Loading state -->
    <div v-if="categoryStore.isLoading" class="py-4 text-center text-[13px] text-secondary">
      Loading…
    </div>

    <!-- Empty state -->
    <div
      v-else-if="categoryStore.data.length === 0"
      class="rounded-[9px] border border-dashed border-border py-8 text-center"
    >
      <p class="text-[13px] text-secondary">No categories yet.</p>
      <button
        type="button"
        class="mt-2 text-[13px] font-semibold text-accent hover:underline"
        @click="openAdd"
      >
        Add your first category
      </button>
    </div>

    <!-- Grouped category list -->
    <ul v-else class="flex flex-col divide-y divide-border/60" role="list">
      <li
        v-for="parent in categoryStore.topLevel"
        :key="parent.id"
        class="py-[3px] first:pt-0 last:pb-0"
      >
        <!-- Parent row -->
        <div class="group flex items-center gap-3 py-[8px]">
          <!-- Color dot -->
          <span
            class="h-3 w-3 flex-none rounded-full border border-black/10"
            :style="{ backgroundColor: parent.colorCode ?? '#6b7280' }"
            aria-hidden="true"
          />

          <!-- Name -->
          <span class="min-w-0 flex-1 truncate text-[13px] font-semibold text-primary">
            {{ parent.name }}
          </span>

          <!-- Type badge -->
          <span
            class="inline-flex items-center rounded-pill px-2 py-[3px] text-[10px] font-semibold capitalize"
            :class="typeBadgeClass(parent.type)"
          >
            {{ parent.type }}
          </span>

          <!-- Row actions -->
          <div
            class="ml-1 flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
          >
            <button
              type="button"
              class="rounded-[6px] border border-border bg-card px-2.5 py-1.5 text-[11px] font-semibold text-secondary hover:text-primary transition-colors"
              :aria-label="`Edit ${parent.name}`"
              @click="openEdit(parent)"
            >
              Edit
            </button>
            <button
              type="button"
              class="rounded-[6px] border border-red/30 bg-red/5 px-2.5 py-1.5 text-[11px] font-semibold text-red hover:bg-red/10 transition-colors"
              :aria-label="`Delete ${parent.name}`"
              @click="requestDelete(parent)"
            >
              Delete
            </button>
          </div>
        </div>

        <!-- Sub-category rows (indented) -->
        <ul
          v-if="categoryStore.childrenOf(parent.id).length > 0"
          class="ml-5 border-l border-border/50 pl-3 pb-1"
          role="list"
        >
          <li
            v-for="child in categoryStore.childrenOf(parent.id)"
            :key="child.id"
            class="group flex items-center gap-3 py-[7px]"
          >
            <!-- Color dot (smaller) -->
            <span
              class="h-[9px] w-[9px] flex-none rounded-full border border-black/10"
              :style="{ backgroundColor: child.colorCode ?? '#6b7280' }"
              aria-hidden="true"
            />

            <!-- Name -->
            <span class="min-w-0 flex-1 truncate text-[12px] font-medium text-primary">
              {{ child.name }}
            </span>

            <!-- Row actions -->
            <div
              class="ml-1 flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
            >
              <button
                type="button"
                class="rounded-[6px] border border-border bg-card px-2.5 py-1.5 text-[11px] font-semibold text-secondary hover:text-primary transition-colors"
                :aria-label="`Edit ${child.name}`"
                @click="openEdit(child)"
              >
                Edit
              </button>
              <button
                type="button"
                class="rounded-[6px] border border-red/30 bg-red/5 px-2.5 py-1.5 text-[11px] font-semibold text-red hover:bg-red/10 transition-colors"
                :aria-label="`Delete ${child.name}`"
                @click="requestDelete(child)"
              >
                Delete
              </button>
            </div>
          </li>
        </ul>
      </li>
    </ul>
  </div>

  <!-- ── Modals ──────────────────────────────────────────────────────────────── -->
  <CategoryFormModal
    v-if="showFormModal"
    :category="editingCategory"
    :top-level-categories="categoryStore.topLevel"
    :profile-id="getProfileId()"
    @save="handleSave"
    @cancel="closeForm"
  />

  <ConfirmDialog
    v-if="deletingCategory"
    title="Delete category?"
    :message="`Delete &quot;${deletingCategory.name}&quot;? This cannot be undone.`"
    confirm-label="Delete"
    :danger="true"
    @confirm="confirmDelete"
    @cancel="cancelDelete"
  />
</template>
