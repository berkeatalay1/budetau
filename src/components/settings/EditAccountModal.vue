<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Account } from '../../types'

const props = defineProps<{
  account: Account
}>()

const emit = defineEmits<{
  save: [payload: Account]
  cancel: []
}>()

const name = ref(props.account.name)
const type = ref(props.account.type)
const currency = ref(props.account.currency)
const nameError = ref('')

const accountTypes = ['Cash', 'Checking', 'Savings', 'Credit']
const currencies = ['USD', 'EUR', 'GBP', 'CAD', 'AUD', 'JPY', 'CHF']

watch(
  () => props.account,
  (a) => {
    name.value = a.name
    type.value = a.type
    currency.value = a.currency
    nameError.value = ''
  },
)

function handleSave() {
  nameError.value = ''
  if (!name.value.trim()) {
    nameError.value = 'Name is required.'
    return
  }
  emit('save', {
    ...props.account,
    name: name.value.trim(),
    type: type.value,
    currency: currency.value,
  })
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
    aria-labelledby="edit-account-title"
    @click.self="emit('cancel')"
    @keydown="handleKeydown"
  >
    <div class="relative w-full max-w-sm rounded-card bg-card shadow-pop border border-border">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-border">
        <h2 id="edit-account-title" class="text-h2 text-primary">Edit Account</h2>
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
          <label for="edit-acct-name" class="text-caption text-secondary">
            Name <span class="text-red" aria-hidden="true">*</span>
          </label>
          <input
            id="edit-acct-name"
            v-model="name"
            type="text"
            required
            maxlength="100"
            placeholder="e.g. Main Checking"
            class="rounded-control border bg-bg px-3 h-10 text-sm text-primary outline-none focus:ring-2 focus:ring-accent/20 transition-shadow placeholder:text-secondary/50"
            :class="nameError ? 'border-red focus:border-red' : 'border-border focus:border-accent'"
            aria-required="true"
            :aria-invalid="!!nameError"
            :aria-describedby="nameError ? 'edit-acct-name-error' : undefined"
          />
          <p v-if="nameError" id="edit-acct-name-error" class="text-[11px] text-red">
            {{ nameError }}
          </p>
        </div>

        <!-- Type -->
        <div class="flex flex-col gap-1.5">
          <label class="text-caption text-secondary">Type</label>
          <div class="flex gap-2 flex-wrap">
            <button
              v-for="t in accountTypes"
              :key="t"
              type="button"
              class="flex-1 min-w-[60px] rounded-control border py-2 text-caption font-semibold capitalize transition-colors"
              :class="
                type === t
                  ? 'border-accent bg-accent/10 text-accent'
                  : 'border-border bg-bg text-secondary hover:bg-bg/80'
              "
              :aria-pressed="type === t"
              @click="type = t"
            >
              {{ t }}
            </button>
          </div>
        </div>

        <!-- Currency -->
        <div class="flex flex-col gap-1.5">
          <label for="edit-acct-currency" class="text-caption text-secondary">Currency</label>
          <select
            id="edit-acct-currency"
            v-model="currency"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option v-for="c in currencies" :key="c" :value="c">{{ c }}</option>
          </select>
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
