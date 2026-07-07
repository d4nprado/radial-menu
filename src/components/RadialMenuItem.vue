<script setup lang="ts">
import { computed } from 'vue'
import { itemStreamToggleState } from '../composables/useObsStreamStatus'
import type { MenuItem, ObsStreamStatus } from '../types/menu'
import MenuItemIcon from './MenuItemIcon.vue'

const props = defineProps<{
  item: MenuItem
  index: number
  total: number
  disabled?: boolean
  menuSize?: number
  obsStreamStatus?: ObsStreamStatus | null
}>()

defineEmits<{
  select: [item: MenuItem]
}>()

const position = computed(() => {
  const angle = (props.index / props.total) * Math.PI * 2 - Math.PI / 2
  const sizeRatio = Math.min(100, Math.max(0, props.menuSize ?? 0)) / 100
  const radius = 160 + sizeRatio * 50
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

const streamState = computed(() =>
  itemStreamToggleState(props.item, props.obsStreamStatus ?? null),
)
const displayHint = computed(() => streamState.value?.hint ?? props.item.hint)
const showStreamLed = computed(() =>
  Boolean(streamState.value?.active || streamState.value?.showInactiveLed),
)
</script>

<template>
  <button
    class="radial-item"
    :class="{ 'has-stream-status': streamState, 'is-stream-active': streamState?.active }"
    :style="position"
    :disabled="disabled"
    :aria-label="`${item.label}: ${displayHint}`"
    @click.stop="$emit('select', item)"
  >
    <span class="radial-item__glow" />
    <span
      v-if="showStreamLed"
      class="radial-item__stream-led"
      aria-hidden="true"
    />
    <span class="radial-item__icon">
      <MenuItemIcon :item="item" :fixed-icon-override="streamState?.icon" />
    </span>
    <span
      class="radial-item__copy"
      :class="`is-${tooltipPlacement}`"
    >
      <strong>{{ item.label }}</strong>
      <small>{{ displayHint }}</small>
    </span>
  </button>
</template>
