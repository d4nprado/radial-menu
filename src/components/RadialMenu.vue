<script setup lang="ts">
import type { CenterAction, MenuItem } from '../types/menu'
import type { SystemStats } from '../types/systemStats'
import RadialMenuItem from './RadialMenuItem.vue'
import SystemStatus from './SystemStatus.vue'

withDefaults(defineProps<{
  items: MenuItem[]
  phase: 'entering' | 'visible' | 'leaving'
  stats: SystemStats | null
  disabled?: boolean
  centerAction?: CenterAction
}>(), {
  centerAction: 'close',
})

defineEmits<{
  select: [item: MenuItem]
  dismiss: []
  centerAction: [action: CenterAction]
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
