<script setup lang="ts">
import { computed } from 'vue'
import type { MenuAction, MenuItem } from '../types/menu'

const props = defineProps<{ items: MenuItem[]; selectedId: string | null }>()
const emit = defineEmits<{ select: [id: string]; edit: [item: MenuItem]; remove: [id: string] }>()
const selected = computed(() => props.items.find((item) => item.id === props.selectedId))
const labels: Record<MenuAction['type'], string> = { program: 'Programa', directory: 'Diretório', url: 'URL', system: 'Sistema' }

function value(action: MenuAction) {
  if (action.type === 'program' || action.type === 'directory') return action.path
  if (action.type === 'url') return action.url
  return action.target.replace('_', ' ')
}
</script>

<template>
  <section class="panel">
    <header><div><small>AÇÕES CONFIGURADAS</small><h2>Itens do menu</h2></div><b>{{ items.length }}</b></header>
    <div v-if="items.length" class="list">
      <button v-for="item in items" :key="item.id" :class="{ selected: item.id === selectedId }" @click="emit('select', item.id)">
        <span class="icon" :style="{ '--accent': item.accent }">{{ item.icon }}</span>
        <span class="copy"><strong>{{ item.label }}</strong><small>{{ labels[item.action.type] }} · {{ value(item.action) }}</small></span>
        <span class="arrow">›</span>
      </button>
    </div>
    <div v-else class="empty">Nenhuma ação configurada</div>
    <footer>
      <button :disabled="!selected" @click="selected && emit('edit', selected)">Editar</button>
      <button class="danger" :disabled="!selected" @click="selected && emit('remove', selected.id)">Remover</button>
    </footer>
  </section>
</template>

<style scoped>
.panel{display:flex;min-height:0;padding:24px;flex-direction:column;border:1px solid #ffffff14;border-radius:22px;background:#111422e6;box-shadow:0 24px 70px #0003}header{display:flex;margin-bottom:18px;align-items:center;justify-content:space-between}header small{color:#8d82ff;font-size:10px;font-weight:700;letter-spacing:.14em}h2{margin:4px 0 0;font-size:20px}header b{display:grid;width:29px;height:29px;place-items:center;border-radius:9px;color:#a5a9bb;background:#ffffff0d;font-size:11px}.list{min-height:0;padding-right:4px;overflow-y:auto}.list>button{display:flex;width:100%;padding:10px;align-items:center;border:1px solid transparent;border-radius:13px;color:#eef0fa;text-align:left;background:transparent;cursor:pointer;gap:11px}.list>button+button{margin-top:5px}.list>button:hover{background:#ffffff0a}.list>button.selected{border-color:#8b7cff3d;background:#8b7cff17}.icon{display:grid;width:39px;height:39px;flex:none;place-items:center;border:1px solid color-mix(in srgb,var(--accent) 38%,transparent);border-radius:12px;color:color-mix(in srgb,var(--accent) 66%,white);background:color-mix(in srgb,var(--accent) 10%,transparent);font-size:11px;font-weight:700}.copy{display:flex;min-width:0;flex:1;flex-direction:column;gap:4px}.copy strong,.copy small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.copy strong{font-size:12px}.copy small{color:#777c91;font-size:9px}.arrow{color:#555b70;font-size:22px}.empty{display:grid;flex:1;place-items:center;color:#777c91;font-size:12px}footer{display:flex;padding-top:16px;border-top:1px solid #ffffff12;gap:9px}footer button{padding:9px 16px;border:1px solid #ffffff1c;border-radius:9px;color:#d8dae5;background:#ffffff0d;cursor:pointer}.danger{border-color:#ff647e33;color:#ff9bae;background:#ff507012}button:disabled{cursor:not-allowed;opacity:.38}
</style>
