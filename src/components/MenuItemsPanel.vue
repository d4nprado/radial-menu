<script setup lang="ts">
import { computed } from 'vue'
import type { MenuAction, MenuItem, SystemActionTarget } from '../types/menu'

const props = defineProps<{
  items: MenuItem[]
  selectedId: string | null
}>()

const emit = defineEmits<{
  select: [id: string]
  edit: [item: MenuItem]
  remove: [id: string]
}>()

const selectedItem = computed(() =>
  props.items.find((item) => item.id === props.selectedId),
)

const actionLabels: Record<MenuAction['type'], string> = {
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

function actionValue(action: MenuAction) {
  if (action.type === 'program' || action.type === 'directory') return action.path
  if (action.type === 'url') return action.url
  return systemLabels[action.target]
}
</script>

<template>
  <section class="items-panel" aria-labelledby="items-title">
    <div class="items-panel__heading">
      <div>
        <span>AÇÕES CONFIGURADAS</span>
        <h2 id="items-title">Itens do menu</h2>
      </div>
      <small>{{ items.length }}</small>
    </div>

    <div v-if="items.length" class="items-panel__list">
      <button
        v-for="item in items"
        :key="item.id"
        type="button"
        class="items-panel__item"
        :class="{ 'is-selected': item.id === selectedId }"
        @click="emit('select', item.id)"
      >
        <span class="items-panel__icon" :style="{ '--item-accent': item.accent }">
          {{ item.icon }}
        </span>
        <span class="items-panel__copy">
          <strong>{{ item.label }}</strong>
          <small>{{ actionLabels[item.action.type] }} · {{ actionValue(item.action) }}</small>
        </span>
        <span class="items-panel__chevron" aria-hidden="true">›</span>
      </button>
    </div>

    <div v-else class="items-panel__empty">
      <strong>Nenhuma ação configurada</strong>
      <span>Use “Adicionar ação” no preview para começar.</span>
    </div>

    <div class="items-panel__actions">
      <button
        type="button"
        class="button-secondary"
        :disabled="!selectedItem"
        @click="selectedItem && emit('edit', selectedItem)"
      >
        Editar
      </button>
      <button
        type="button"
        class="button-danger"
        :disabled="!selectedItem"
        @click="selectedItem && emit('remove', selectedItem.id)"
      >
        Remover
      </button>
    </div>
  </section>
</template>

<style scoped>
.items-panel {
  display: flex;
  min-height: 0;
  padding: 24px;
  flex-direction: column;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 22px;
  background: rgb(17 20 34 / 90%);
  box-shadow: 0 24px 70px rgb(0 0 0 / 20%);
}

.items-panel__heading {
  display: flex;
  margin-bottom: 18px;
  align-items: center;
  justify-content: space-between;
}

.items-panel__heading span {
  color: #8d82ff;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.14em;
}

h2 {
  margin: 4px 0 0;
  font-size: 20px;
}

.items-panel__heading small {
  display: grid;
  width: 29px;
  height: 29px;
  place-items: center;
  border-radius: 9px;
  color: #a5a9bb;
  background: rgb(255 255 255 / 5%);
  font-size: 11px;
}

.items-panel__list {
  min-height: 0;
  padding-right: 4px;
  overflow-y: auto;
}

.items-panel__item {
  display: flex;
  width: 100%;
  padding: 10px;
  align-items: center;
  border: 1px solid transparent;
  border-radius: 13px;
  color: #eef0fa;
  text-align: left;
  background: transparent;
  cursor: pointer;
  gap: 11px;
}

.items-panel__item + .items-panel__item {
  margin-top: 5px;
}

.items-panel__item:hover {
  background: rgb(255 255 255 / 4%);
}

.items-panel__item.is-selected {
  border-color: rgb(139 124 255 / 24%);
  background: rgb(139 124 255 / 9%);
}

.items-panel__icon {
  --item-accent: #8b7cff;
  display: grid;
  width: 39px;
  height: 39px;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--item-accent) 38%, transparent);
  border-radius: 12px;
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
  gap: 4px;
}

.items-panel__copy strong,
.items-panel__copy small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.items-panel__copy strong {
  font-size: 12px;
}

.items-panel__copy small {
  color: #777c91;
  font-size: 9px;
}

.items-panel__chevron {
  color: #555b70;
  font-size: 22px;
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

.items-panel__actions {
  display: flex;
  padding-top: 16px;
  border-top: 1px solid rgb(255 255 255 / 7%);
  gap: 9px;
}

.items-panel__actions button {
  padding: 9px 16px;
  border-radius: 9px;
  cursor: pointer;
}

.button-secondary {
  border: 1px solid rgb(255 255 255 / 11%);
  color: #d8dae5;
  background: rgb(255 255 255 / 5%);
}

.button-danger {
  border: 1px solid rgb(255 100 126 / 20%);
  color: #ff9bae;
  background: rgb(255 80 112 / 7%);
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.38;
}
</style>
