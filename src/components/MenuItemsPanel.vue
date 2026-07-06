<script setup lang="ts">
import type { MenuAction, MenuItem, SystemActionTarget } from '../types/menu'

defineProps<{
  items: MenuItem[]
  selectedId: string | null
  groupLabel: string | null
}>()

const emit = defineEmits<{
  select: [id: string]
  edit: [item: MenuItem]
  remove: [id: string]
  openGroup: [id: string]
  back: []
}>()

const actionLabels: Record<Exclude<MenuAction['type'], 'group'>, string> = {
  program: 'Programa',
  directory: 'Diretório',
  url: 'URL',
  system: 'Sistema',
}

const systemLabels: Record<SystemActionTarget, string> = {
  explorer: 'Explorador de arquivos',
  default_browser: 'Navegador padrão',
  terminal: 'Terminal',
  calculator: 'Calculadora',
  notepad: 'Bloco de notas',
}

function itemSubtitle(item: MenuItem) {
  const action = item.action
  if (action.type === 'group') {
    const count = action.items.length
    return `${count} ${count === 1 ? 'item' : 'itens'}`
  }
  if (action.type === 'program' || action.type === 'directory') {
    return `${actionLabels[action.type]} · ${action.path}`
  }
  if (action.type === 'url') return `${actionLabels.url} · ${action.url}`
  return `${actionLabels.system} · ${systemLabels[action.target]}`
}
</script>

<template>
  <section class="items-panel" aria-labelledby="items-title">
    <div v-if="groupLabel" class="items-panel__breadcrumb">
      <button type="button" @click="emit('back')">← Voltar</button>
      <span>Grupo: <strong>{{ groupLabel }}</strong></span>
    </div>

    <div class="items-panel__heading">
      <div>
        <span>{{ groupLabel ? 'ITENS DO GRUPO' : 'MENU PRINCIPAL' }}</span>
        <h2 id="items-title">{{ groupLabel ? groupLabel : 'Itens do menu' }}</h2>
      </div>
      <small>{{ items.length }}/10</small>
    </div>

    <div v-if="items.length" class="items-panel__list">
      <article
        v-for="item in items"
        :key="item.id"
        class="items-panel__item"
        :class="{ 'is-selected': item.id === selectedId }"
        @click="emit('select', item.id)"
      >
        <span class="items-panel__icon" :style="{ '--item-accent': item.accent }">
          {{ item.icon }}
        </span>

        <span class="items-panel__copy">
          <span class="items-panel__title">
            <strong>{{ item.label }}</strong>
            <em :class="{ 'is-group': item.action.type === 'group' }">
              {{ item.action.type === 'group' ? 'Grupo' : 'Ação' }}
            </em>
          </span>
          <small>{{ itemSubtitle(item) }}</small>
          <span class="items-panel__controls">
            <button type="button" @click.stop="emit('edit', item)">Editar</button>
            <button
              type="button"
              class="is-danger"
              @click.stop="emit('remove', item.id)"
            >
              Excluir
            </button>
            <button
              v-if="item.action.type === 'group'"
              type="button"
              class="is-open"
              @click.stop="emit('openGroup', item.id)"
            >
              Abrir grupo
            </button>
          </span>
        </span>
      </article>
    </div>

    <div v-else class="items-panel__empty">
      <strong>{{ groupLabel ? 'Este grupo está vazio' : 'Nenhum item configurado' }}</strong>
      <span>Use “Adicionar ação” no preview para começar.</span>
    </div>
  </section>
</template>

<style scoped>
.items-panel {
  display: flex;
  min-height: 0;
  padding: 20px;
  flex-direction: column;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 22px;
  background: rgb(17 20 34 / 90%);
  box-shadow: 0 24px 70px rgb(0 0 0 / 20%);
}

.items-panel__breadcrumb {
  display: flex;
  margin-bottom: 8px;
  padding-bottom: 8px;
  align-items: center;
  border-bottom: 1px solid rgb(255 255 255 / 7%);
  gap: 10px;
}

.items-panel__breadcrumb button {
  padding: 4px 8px;
  border: 1px solid rgb(139 124 255 / 22%);
  border-radius: 8px;
  color: #bdb7ff;
  background: rgb(139 124 255 / 8%);
  cursor: pointer;
}

.items-panel__breadcrumb span {
  overflow: hidden;
  color: #777c91;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.items-panel__breadcrumb strong {
  color: #c8c5d8;
}

.items-panel__heading {
  display: flex;
  margin-bottom: 13px;
  align-items: center;
  justify-content: space-between;
}

.items-panel__heading span {
  color: #8d82ff;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.14em;
}

h2 {
  margin: 3px 0 0;
  font-size: 18px;
}

.items-panel__heading small {
  padding: 6px 8px;
  border-radius: 8px;
  color: #a5a9bb;
  background: rgb(255 255 255 / 5%);
  font-size: 10px;
}

.items-panel__list {
  min-height: 0;
  padding-right: 4px;
  overflow-y: auto;
  scrollbar-color: rgb(139 124 255 / 35%) rgb(255 255 255 / 4%);
  scrollbar-width: thin;
}

.items-panel__list::-webkit-scrollbar {
  width: 7px;
}

.items-panel__list::-webkit-scrollbar-track {
  border-radius: 999px;
  background: rgb(255 255 255 / 4%);
}

.items-panel__list::-webkit-scrollbar-thumb {
  border-radius: 999px;
  background: rgb(139 124 255 / 35%);
}

.items-panel__item {
  display: flex;
  min-height: 55px;
  padding: 6px 9px;
  align-items: center;
  border: 1px solid rgb(255 255 255 / 5%);
  border-radius: 12px;
  color: #eef0fa;
  background: rgb(255 255 255 / 2%);
  cursor: pointer;
  gap: 10px;
}

.items-panel__item + .items-panel__item {
  margin-top: 5px;
}

.items-panel__item:hover {
  border-color: rgb(255 255 255 / 10%);
  background: rgb(255 255 255 / 4%);
}

.items-panel__item.is-selected {
  border-color: rgb(139 124 255 / 30%);
  background: rgb(139 124 255 / 8%);
}

.items-panel__icon {
  --item-accent: #8b7cff;
  display: grid;
  width: 38px;
  height: 38px;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--item-accent) 38%, transparent);
  border-radius: 11px;
  color: color-mix(in srgb, var(--item-accent) 66%, white);
  background: color-mix(in srgb, var(--item-accent) 10%, transparent);
  font-size: 11px;
  font-weight: 700;
}

.items-panel__copy {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 3px;
}

.items-panel__title,
.items-panel__controls {
  display: flex;
  align-items: center;
}

.items-panel__title {
  justify-content: space-between;
  gap: 8px;
}

.items-panel__title strong,
.items-panel__copy > small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.items-panel__title strong {
  font-size: 11px;
}

.items-panel__title em {
  padding: 3px 6px;
  flex: 0 0 auto;
  border-radius: 999px;
  color: #858a9e;
  background: rgb(255 255 255 / 5%);
  font-size: 7px;
  font-style: normal;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.items-panel__title em.is-group {
  color: #bcb6ff;
  background: rgb(139 124 255 / 11%);
}

.items-panel__copy > small {
  color: #777c91;
  font-size: 8px;
}

.items-panel__controls {
  gap: 5px;
}

.items-panel__controls button {
  padding: 3px 6px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 6px;
  color: #aeb2c2;
  background: rgb(255 255 255 / 3%);
  font-size: 7px;
  cursor: pointer;
}

.items-panel__controls button:hover {
  color: #e2e4ee;
  border-color: rgb(255 255 255 / 15%);
}

.items-panel__controls button.is-danger {
  color: #ff9bae;
  border-color: rgb(255 100 126 / 14%);
}

.items-panel__controls button.is-open {
  color: #bdb7ff;
  border-color: rgb(139 124 255 / 20%);
}

.items-panel__empty {
  display: grid;
  min-height: 180px;
  flex: 1;
  place-content: center;
  color: #747a90;
  text-align: center;
  gap: 7px;
}

.items-panel__empty strong {
  color: #aeb2c2;
  font-size: 13px;
}

.items-panel__empty span {
  font-size: 10px;
}
</style>
