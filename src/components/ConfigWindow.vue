<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, toRaw } from 'vue'
import { useObsStreamStatus } from '../composables/useObsStreamStatus'
import {
  MAX_MENU_ITEMS_PER_LEVEL,
  type ConfigLoadResponse,
  type MenuItem,
} from '../types/menu'
import AppPreferencesModal from './AppPreferencesModal.vue'
import MenuItemFormModal from './MenuItemFormModal.vue'
import MenuItemsPanel from './MenuItemsPanel.vue'
import RadialMenuPreview from './RadialMenuPreview.vue'

function cloneItems(items: MenuItem[]) {
  return structuredClone(toRaw(items))
}

const shortcut = ref('Ctrl+Space')
const radialMenuSize = ref(0)
const items = ref<MenuItem[]>([])
const currentGroupId = ref<string | null>(null)
const currentGroup = computed(() => {
  const item = items.value.find((candidate) => candidate.id === currentGroupId.value)
  return item?.action.type === 'group' ? item : null
})
const currentItems = computed(() =>
  currentGroup.value?.action.type === 'group'
    ? currentGroup.value.action.items
    : items.value,
)
const selectedId = ref<string | null>(null)
const formMode = ref<'add' | 'edit' | null>(null)
const editingItem = ref<MenuItem | null>(null)
const editingOriginalId = ref<string | null>(null)
const showPreferences = ref(false)
const busy = ref(true)
const status = ref('Carregando configuração…')
const statusKind = ref<'neutral' | 'success' | 'error'>('neutral')
const {
  status: obsStreamStatus,
  refresh: refreshObsStreamStatus,
} = useObsStreamStatus()

function setStatus(message: string, kind: 'neutral' | 'success' | 'error' = 'neutral') {
  status.value = message
  statusKind.value = kind
}

function hasStreamToggle(menuItems: MenuItem[]): boolean {
  return menuItems.some((item) => {
    if (item.action.type === 'group') return hasStreamToggle(item.action.items)
    return item.action.type === 'stream'
      && (item.action.operation === 'toggle_recording'
        || item.action.operation === 'toggle_streaming'
        || item.action.operation === 'toggle_input_mute')
  })
}

function obsToggleInputNames(menuItems: MenuItem[]): string[] {
  return menuItems.flatMap((item) => {
    if (item.action.type === 'group') return obsToggleInputNames(item.action.items)
    if (
      item.action.type === 'stream'
      && item.action.operation === 'toggle_input_mute'
      && item.action.inputName?.trim()
    ) {
      return [item.action.inputName.trim()]
    }
    return []
  })
}

async function loadConfig() {
  busy.value = true
  try {
    const response = await invoke<ConfigLoadResponse>('load_launcher_config')
    shortcut.value = response.config.shortcut
    radialMenuSize.value = response.config.radialMenuSize ?? 0
    items.value = cloneItems(response.config.items)
    currentGroupId.value = null
    selectedId.value = items.value[0]?.id ?? null
    if (hasStreamToggle(items.value)) void refreshObsStreamStatus(obsToggleInputNames(items.value))
    setStatus(response.warning ?? 'Configuração carregada', response.warning ? 'error' : 'neutral')
  } catch (cause) {
    items.value = []
    setStatus(
      typeof cause === 'string' ? cause : 'Não foi possível carregar. Usando o padrão.',
      'error',
    )
  } finally {
    busy.value = false
  }
}

function selectItem(id: string) {
  selectedId.value = id
}

function openAddForm() {
  if (currentItems.value.length >= MAX_MENU_ITEMS_PER_LEVEL) {
    setStatus('Limite de 10 itens neste nível', 'error')
    return
  }

  editingItem.value = null
  editingOriginalId.value = null
  formMode.value = 'add'
}

function openEditForm(item: MenuItem) {
  editingItem.value = structuredClone(toRaw(item))
  editingOriginalId.value = item.id
  formMode.value = 'edit'
}

function closeForm() {
  formMode.value = null
  editingItem.value = null
  editingOriginalId.value = null
}

function createItemId(label: string) {
  const base = label
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'acao'
  let id = base
  let suffix = 2

  while (currentItems.value.some((item) => item.id === id)) {
    id = `${base}-${suffix}`
    suffix += 1
  }
  return id
}

function saveItem(item: MenuItem) {
  if (formMode.value === 'edit') {
    const originalId = editingOriginalId.value
    const index = currentItems.value.findIndex((current) => current.id === originalId)
    if (index < 0 || !originalId) {
      setStatus('O item que estava sendo editado não foi encontrado.', 'error')
      return
    }
    item.id = originalId
    currentItems.value[index] = item
  } else {
    if (currentItems.value.length >= MAX_MENU_ITEMS_PER_LEVEL) {
      setStatus('Limite de 10 itens neste nível', 'error')
      closeForm()
      return
    }
    item.id = createItemId(item.label)
    currentItems.value.push(item)
  }

  selectedId.value = item.id
  setStatus('Alterações ainda não salvas')
  closeForm()
}

function reorderItems(fromIndex: number, toIndex: number) {
  if (
    fromIndex === toIndex
    || fromIndex < 0
    || toIndex < 0
    || fromIndex >= currentItems.value.length
    || toIndex >= currentItems.value.length
  ) return

  const [movedItem] = currentItems.value.splice(fromIndex, 1)
  if (!movedItem) return
  currentItems.value.splice(toIndex, 0, movedItem)
  selectedId.value = movedItem.id
  setStatus('Alterações ainda não salvas')
}

function removeItem(id: string) {
  const index = currentItems.value.findIndex((item) => item.id === id)
  if (index < 0) return

  const item = currentItems.value[index]
  if (
    item?.action.type === 'group'
    && !window.confirm(
      item.action.items.length === 1
        ? `Excluir o grupo “${item.label}”? O item dentro dele também será removido.`
        : `Excluir o grupo “${item.label}”? Os ${item.action.items.length} itens dentro dele também serão removidos.`,
    )
  ) return

  currentItems.value.splice(index, 1)
  selectedId.value =
    currentItems.value[index]?.id
    ?? currentItems.value[index - 1]?.id
    ?? null
  setStatus('Alterações ainda não salvas')
}

function openGroup(id: string) {
  const group = items.value.find((item) => item.id === id)
  if (group?.action.type !== 'group') return

  currentGroupId.value = group.id
  selectedId.value = group.action.items[0]?.id ?? null
}

function closeGroup() {
  const groupId = currentGroupId.value
  currentGroupId.value = null
  selectedId.value = groupId
}

function updateRadialMenuSize(size: number) {
  radialMenuSize.value = size
  setStatus('Alterações ainda não salvas')
}

async function saveConfig() {
  busy.value = true
  try {
    await invoke('save_launcher_config', {
      config: {
        shortcut: shortcut.value,
        radialMenuSize: radialMenuSize.value,
        items: items.value,
      },
    })
    setStatus('Configuração salva', 'success')
  } catch (cause) {
    setStatus(
      typeof cause === 'string' ? cause : 'Não foi possível salvar a configuração.',
      'error',
    )
  } finally {
    busy.value = false
  }
}

onMounted(loadConfig)
</script>

<template>
  <main class="config-window">
    <header class="config-header">
      <div class="config-header__brand">
        <span class="config-header__mark">O</span>
        <div>
          <h1>Orbit Launcher</h1>
          <p>Configure os atalhos do menu radial</p>
        </div>
      </div>
      <button
        type="button"
        class="config-header__settings"
        aria-label="Configurações do app"
        title="Configurações do app"
        @click="showPreferences = true"
      >
        ⚙
      </button>
    </header>

    <div class="config-content">
      <RadialMenuPreview
        :items="currentItems"
        :selected-id="selectedId"
        :max-items="MAX_MENU_ITEMS_PER_LEVEL"
        :group-label="currentGroup?.label ?? null"
        :menu-size="radialMenuSize"
        :obs-stream-status="obsStreamStatus"
        @select="selectItem"
        @add="openAddForm"
        @reorder="reorderItems"
        @open-group="openGroup"
        @back="closeGroup"
        @update:menu-size="updateRadialMenuSize"
      />
      <MenuItemsPanel
        :items="currentItems"
        :selected-id="selectedId"
        :group-label="currentGroup?.label ?? null"
        @select="selectItem"
        @edit="openEditForm"
        @remove="removeItem"
        @open-group="openGroup"
        @back="closeGroup"
      />
    </div>

    <footer class="config-footer">
      <span class="config-footer__status" :class="{ 'is-error': statusKind === 'error' }">
        <i :class="{ 'is-saved': statusKind === 'success' }" />
        {{ status }}
      </span>
      <div>
        <button type="button" class="button-primary" :disabled="busy" @click="saveConfig">
          Salvar
        </button>
      </div>
    </footer>

    <MenuItemFormModal
      v-if="formMode"
      :item="editingItem"
      :allow-groups="!currentGroup"
      @save="saveItem"
      @cancel="closeForm"
    />

    <AppPreferencesModal v-if="showPreferences" @close="showPreferences = false" />
  </main>
</template>

<style scoped>
.config-window {
  display: grid;
  min-width: 1080px;
  min-height: 740px;
  padding: 0 34px 22px;
  overflow: auto;
  grid-template-rows: 92px minmax(0, 1fr) 70px;
  color: #f4f5fb;
  background:
    radial-gradient(circle at 12% 0%, rgb(93 74 212 / 16%), transparent 28%),
    radial-gradient(circle at 95% 90%, rgb(61 152 190 / 8%), transparent 26%),
    #090b14;
}

.config-header,
.config-header__brand,
.config-footer,
.config-footer > div,
.config-footer__status {
  display: flex;
  align-items: center;
}

.config-header {
  justify-content: space-between;
  border-bottom: 1px solid rgb(255 255 255 / 7%);
}

.config-header__brand {
  gap: 13px;
}

.config-header__mark {
  display: grid;
  width: 39px;
  height: 39px;
  place-items: center;
  border: 1px solid rgb(139 124 255 / 45%);
  border-radius: 13px;
  color: #c5bfff;
  background: linear-gradient(145deg, rgb(139 124 255 / 26%), rgb(89 74 196 / 10%));
  box-shadow: 0 0 24px rgb(139 124 255 / 16%);
  font-size: 17px;
  font-weight: 700;
}

h1 {
  margin: 0;
  font-size: 18px;
  letter-spacing: -0.02em;
}

.config-header p {
  margin: 4px 0 0;
  color: #777c91;
  font-size: 10px;
}

.config-header__settings {
  display: grid;
  width: 38px;
  height: 38px;
  padding: 0;
  place-items: center;
  border: 1px solid rgb(255 255 255 / 9%);
  border-radius: 11px;
  color: #a7aabc;
  background: rgb(255 255 255 / 4%);
  font-size: 17px;
  cursor: pointer;
}

.config-header__settings:hover {
  color: #d2ceff;
  border-color: rgb(139 124 255 / 30%);
  background: rgb(139 124 255 / 8%);
}

.config-content {
  display: grid;
  min-height: 0;
  padding: 24px 0;
  grid-template-columns: minmax(560px, 1.25fr) minmax(420px, 0.75fr);
  gap: 20px;
}

.config-footer {
  justify-content: space-between;
  border-top: 1px solid rgb(255 255 255 / 7%);
}

.config-footer__status {
  color: #777c91;
  font-size: 10px;
  gap: 8px;
}

.config-footer__status i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #f0b85e;
  box-shadow: 0 0 8px rgb(240 184 94 / 50%);
}

.config-footer__status i.is-saved {
  background: #55d6be;
  box-shadow: 0 0 8px rgb(85 214 190 / 58%);
}

.config-footer__status.is-error {
  color: #ff9bae;
}

.config-footer > div {
  gap: 9px;
}

.config-footer button {
  padding: 10px 17px;
  border-radius: 10px;
  cursor: pointer;
}

.button-primary {
  min-width: 86px;
  border: 1px solid #7669e2;
  color: white;
  background: linear-gradient(135deg, #796bea, #5c4fc8);
  box-shadow: 0 8px 20px rgb(91 77 202 / 20%);
}

.simple-modal-backdrop {
  position: fixed;
  z-index: 40;
  inset: 0;
  display: grid;
  padding: 30px;
  place-items: center;
  background: rgb(3 5 11 / 72%);
  backdrop-filter: blur(8px);
}

.simple-modal {
  width: min(100%, 390px);
  padding: 32px;
  border: 1px solid rgb(255 255 255 / 10%);
  border-radius: 20px;
  text-align: center;
  background: #171a29;
  box-shadow: 0 28px 80px rgb(0 0 0 / 48%);
}

.simple-modal > span {
  display: grid;
  width: 48px;
  height: 48px;
  margin: 0 auto 16px;
  place-items: center;
  border-radius: 15px;
  color: #beb7ff;
  background: rgb(139 124 255 / 12%);
  font-size: 20px;
}

.simple-modal h2 {
  margin: 0;
  font-size: 19px;
}

.simple-modal p {
  margin: 10px 0 22px;
  color: #858a9e;
  font-size: 11px;
  line-height: 1.55;
}

.simple-modal button {
  padding: 9px 17px;
  border: 1px solid rgb(139 124 255 / 34%);
  border-radius: 9px;
  color: #d2ceff;
  background: rgb(139 124 255 / 11%);
  cursor: pointer;
}

@media (max-width: 1140px) {
  .config-content {
    grid-template-columns: minmax(520px, 1fr) minmax(390px, 0.75fr);
  }
}
</style>
