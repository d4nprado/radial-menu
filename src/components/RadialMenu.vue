<script setup lang="ts">
import type { MenuItem } from '../types/menu'
import type { SystemStats } from '../types/systemStats'
import RadialMenuItem from './RadialMenuItem.vue'
import SystemStatus from './SystemStatus.vue'

defineProps<{
  items: MenuItem[]
  shortcut: string
  phase: 'entering' | 'visible' | 'leaving'
  stats: SystemStats | null
  disabled?: boolean
}>()

defineEmits<{
  select: [item: MenuItem]
  dismiss: []
}>()
</script>

<template>
  <div class="menu-stage" :class="`is-${phase}`" @pointerdown.self="$emit('dismiss')">
    <div class="radial-menu" role="menu" aria-label="Menu de atalhos">
      <div class="radial-menu__orbit radial-menu__orbit--outer" />
      <div class="radial-menu__orbit radial-menu__orbit--inner" />

      <RadialMenuItem
        v-for="(item, index) in items"
        :key="item.id"
        :item="item"
        :index="index"
        :total="items.length"
        :disabled="disabled"
        @select="$emit('select', $event)"
      />

      <p v-if="items.length === 0" class="radial-menu__empty">
        Nenhuma ação configurada
      </p>

      <button class="radial-center" aria-label="Fechar menu" @click.stop="$emit('dismiss')">
        <SystemStatus :stats="stats" :shortcut="shortcut" />
      </button>
    </div>
  </div>
</template>
