<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  profileId: number
}>()

const emit = defineEmits<{
  save: [
    payload: { profileId: number; name: string; type: string; currency: string; balance: number },
  ]
  cancel: []
}>()

const name = ref('')
const type = ref('Checking')
const currency = ref('USD')
const openingBalance = ref<number | null>(null)
const nameError = ref('')

const accountTypes = ['Cash', 'Checking', 'Savings', 'Credit']
const currencies = ['USD', 'EUR', 'GBP', 'CAD', 'AUD', 'JPY', 'CHF']

function handleSave() {
  nameError.value = ''
  if (!name.value.trim()) {
    nameError.value = 'Name is required.'
    return
  }
  const balanceCents = openingBalance.value != null ? Math.round(openingBalance.value * 100) : 0
  emit('save', {
    profileId: props.profileId,
    name: name.value.trim(),
    type: type.value,
    currency: currency.value,
    balance: balanceCents,
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
    aria-labelledby="account-form-title"
    @click.self="emit('cancel')"
    @keydown="handleKeydown"
  >
    <div class="relative w-full max-w-sm rounded-card bg-card shadow-pop border border-border">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-border">
        <h2 id="account-form-title" class="text-h2 text-primary">Add Account</h2>
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
          <label for="acct-name" class="text-caption text-secondary"
            >Name <span class="text-red" aria-hidden="true">*</span></label
          >
          <input
            id="acct-name"
            v-model="name"
            type="text"
            required
            maxlength="100"
            placeholder="e.g. Main Checking"
            class="rounded-control border bg-bg px-3 h-10 text-sm text-primary outline-none focus:ring-2 focus:ring-accent/20 transition-shadow placeholder:text-secondary/50"
            :class="nameError ? 'border-red focus:border-red' : 'border-border focus:border-accent'"
            aria-required="true"
            :aria-invalid="!!nameError"
            :aria-describedby="nameError ? 'acct-name-error' : undefined"
          />
          <p v-if="nameError" id="acct-name-error" class="text-[11px] text-red">{{ nameError }}</p>
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
          <label for="acct-currency" class="text-caption text-secondary">Currency</label>
          <select
            id="acct-currency"
            v-model="currency"
            class="rounded-control border border-border bg-bg px-3 h-10 text-sm text-primary outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-shadow"
          >
            <option v-for="c in currencies" :key="c" :value="c">{{ c }}</option>
          </select>
        </div>

        <!-- Opening balance -->
        <div class="flex flex-col gap-1.5">
          <label for="acct-balance" class="text-caption text-secondary"
            >Opening balance <span class="text-secondary/50">(optional)</span></label
          >
          <div
            class="flex items-center gap-2 rounded-control border border-border bg-bg px-3 h-10 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20 transition-shadow"
          >
            <span class="text-secondary text-sm" aria-hidden="true">{{ currency }}</span>
            <input
              id="acct-balance"
              v-model="openingBalance"
              type="number"
              step="0.01"
              min="0"
              class="flex-1 bg-transparent text-sm text-primary outline-none num"
              placeholder="0.00"
            />
          </div>
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
            Add account
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
