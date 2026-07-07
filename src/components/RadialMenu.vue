<script setup lang="ts">
import { computed } from 'vue'
import type { CenterAction, MenuItem } from '../types/menu'
import type { SystemStats } from '../types/systemStats'
import RadialMenuItem from './RadialMenuItem.vue'
import SystemStatus from './SystemStatus.vue'

const props = withDefaults(defineProps<{
  items: MenuItem[]
  phase: 'entering' | 'visible' | 'leaving'
  stats: SystemStats | null
  disabled?: boolean
  centerAction?: CenterAction
  levelKey?: string
  menuSize?: number
}>(), {
  centerAction: 'close',
  levelKey: 'main',
  menuSize: 0,
})

defineEmits<{
  select: [item: MenuItem]
  dismiss: []
  centerAction: [action: CenterAction]
}>()

const sizeRatio = computed(() => Math.min(100, Math.max(0, props.menuSize)) / 100)
const itemRadius = computed(() => 160 + sizeRatio.value * 50)
const itemSize = computed(() => 70 + sizeRatio.value * 28)
const iconSize = computed(() => 36 + sizeRatio.value * 12)
const menuStyle = computed(() => ({
  '--menu-backdrop-inset': `${78 - sizeRatio.value * 44}px`,
  '--menu-outer-size': `${itemRadius.value * 2 + 54}px`,
  '--menu-item-size': `${itemSize.value}px`,
  '--menu-item-radius': `${20 + sizeRatio.value * 7}px`,
  '--menu-item-glow-inset': `${6 + sizeRatio.value * 2}px`,
  '--menu-item-glow-radius': `${16 + sizeRatio.value * 6}px`,
  '--menu-icon-size': `${iconSize.value}px`,
  '--menu-icon-radius': `${12 + sizeRatio.value * 5}px`,
}))
</script>

<template>
  <div class="menu-stage" :class="`is-${phase}`" @pointerdown.self="$emit('dismiss')">
    <div class="radial-menu" role="menu" aria-label="Menu de atalhos" :style="menuStyle">
      <div class="radial-menu__orbit radial-menu__orbit--outer" />
      <div class="radial-menu__orbit radial-menu__orbit--inner" />

      <RadialMenuItem
        v-for="(item, index) in items"
        :key="`${levelKey}:${item.id}`"
        :item="item"
        :index="index"
        :total="items.length"
        :disabled="disabled"
        :menu-size="menuSize"
        @select="$emit('select', $event)"
      />

      <p v-if="items.length === 0" class="radial-menu__empty">
        Nenhuma ação configurada
      </p>

      <button
        class="radial-center"
        :aria-label="centerAction === 'back' ? 'Voltar ao menu anterior' : 'Fechar menu'"
        @click.stop="$emit('centerAction', centerAction)"
      >
        <SystemStatus :stats="stats" :center-action="centerAction" />
      </button>
    </div>
  </div>
</template>
