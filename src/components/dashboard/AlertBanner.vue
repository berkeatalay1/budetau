<script setup lang="ts">
import { computed } from 'vue'
import { formatCents } from '../../lib/format'

const props = defineProps<{
  categoryName: string
  spent: number
  limit: number
  status: 'near' | 'over'
}>()

const pctDisplay = computed(() =>
  props.limit > 0 ? Math.round((props.spent / props.limit) * 100) : 0,
)
const remaining = computed(() => props.limit - props.spent)

const isOver = computed(() => props.status === 'over')

// Amber banner (near)
// Red banner (over)
const wrapClass = computed(() =>
  isOver.value
    ? 'bg-red/[0.07] border border-red/25 border-l-[3px] border-l-red'
    : 'bg-amber/[0.07] border border-amber/25 border-l-[3px] border-l-amber',
)
const dotClass = computed(() => (isOver.value ? 'bg-red text-card' : 'bg-amber text-card'))
const textClass = computed(() => (isOver.value ? 'text-red/90' : 'text-amber/90'))
const strongClass = computed(() =>
  isOver.value ? 'text-red font-semibold' : 'text-amber font-semibold',
)
</script>

<template>
  <div class="flex items-center gap-3 rounded-[10px] px-[14px] py-[11px]" :class="wrapClass">
    <!-- ! dot -->
    <div
      class="w-5 h-5 rounded-full flex-none flex items-center justify-center font-bold text-[12px] leading-none"
      :class="dotClass"
    >
      !
    </div>

    <!-- Message -->
    <div class="flex-1 text-[13px] font-medium leading-[1.4]" :class="textClass">
      <template v-if="isOver">
        <strong :class="strongClass">{{ categoryName }}</strong>
        {{ ' ' }}is over budget — {{ formatCents(spent) }} of {{ formatCents(limit) }} ({{
          pctDisplay
        }}%).
      </template>
      <template v-else>
        <strong :class="strongClass">{{ categoryName }}</strong>
        {{ ' ' }}is at {{ pctDisplay }}% of its {{ formatCents(limit) }} budget ·
        {{ formatCents(remaining) }} left.
      </template>
    </div>
  </div>
</template>
