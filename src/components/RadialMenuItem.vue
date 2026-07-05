<script setup lang="ts">
import { computed } from 'vue'
import type { MenuItem } from '../types/menu'

const props = defineProps<{
  item: MenuItem
  index: number
  total: number
  disabled?: boolean
}>()

defineEmits<{
  select: [item: MenuItem]
}>()

const position = computed(() => {
  const angle = (props.index / props.total) * Math.PI * 2 - Math.PI / 2
  const radius = 170
  const y = Math.sin(angle) * radius

  return {
    '--x': `${Math.cos(angle) * radius}px`,
    '--y': `${y}px`,
    '--accent': props.item.accent,
    '--delay': `${props.index * 28}ms`,
  }
})

const tooltipPlacement = computed(() => {
  const angle = (props.index / props.total) * Math.PI * 2 - Math.PI / 2
  return Math.sin(angle) < 0 ? 'above' : 'below'
})
</script>

<template>
  <button
    class="radial-item"
    :style="position"
    :disabled="disabled"
    :aria-label="`${item.label}: ${item.hint}`"
    @click.stop="$emit('select', item)"
  >
    <span class="radial-item__glow" />
    <span class="radial-item__icon">{{ item.icon }}</span>
    <span
      class="radial-item__copy"
      :class="`is-${tooltipPlacement}`"
    >
      <strong>{{ item.label }}</strong>
      <small>{{ item.hint }}</small>
    </span>
  </button>
</template>
