<script setup lang="ts">
import { ref } from 'vue'
import menuConfigJson from '../config/menu.json'
import type { MenuConfig, MenuItem } from '../types/menu'
import MenuItemFormModal from './MenuItemFormModal.vue'
import MenuItemsPanel from './MenuItemsPanel.vue'
import RadialMenuPreview from './RadialMenuPreview.vue'

const defaultConfig = menuConfigJson as MenuConfig

function cloneItems(items: MenuItem[]) {
  return structuredClone(items)
}

const items = ref<MenuItem[]>(cloneItems(defaultConfig.items))
const selectedId = ref<string | null>(items.value[0]?.id ?? null)
const formMode = ref<'add' | 'edit' | null>(null)
const editingItem = ref<MenuItem | null>(null)
const showAppSettings = ref(false)
const status = ref('Alterações ainda não salvas')
let statusTimer: number | undefined

function selectItem(id: string) {
  selectedId.value = id
}

function openAddForm() {
  editingItem.value = null
  formMode.value = 'add'
}

function openEditForm(item: MenuItem) {
  editingItem.value = cloneItems([item])[0]
  formMode.value = 'edit'
}

function closeForm() {
  formMode.value = null
  editingItem.value = null
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

  while (items.value.some((item) => item.id === id)) {
    id = `${base}-${suffix}`
    suffix += 1
  }
  return id
}

function saveItem(item: MenuItem) {
  if (formMode.value === 'edit') {
    const index = items.value.findIndex((current) => current.id === item.id)
    if (index >= 0) items.value[index] = item
  } else {
    item.id = createItemId(item.label)
    items.value.push(item)
  }

  selectedId.value = item.id
  status.value = 'Alterações ainda não salvas'
  closeForm()
}

function removeItem(id: string) {
  const index = items.value.findIndex((item) => item.id === id)
  if (index < 0) return

  items.value.splice(index, 1)
  selectedId.value = items.value[index]?.id ?? items.value[index - 1]?.id ?? null
  status.value = 'Alterações ainda não salvas'
}

function saveSession() {
  window.clearTimeout(statusTimer)
  status.value = 'Salvo com sucesso nesta sessão'
  statusTimer = window.setTimeout(() => {
    status.value = 'Sem persistência permanente nesta etapa'
  }, 3000)
}

function restoreDefaults() {
  items.value = cloneItems(defaultConfig.items)
  selectedId.value = items.value[0]?.id ?? null
  status.value = 'Padrão restaurado — clique em Salvar'
}
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
        @click="showAppSettings = true"
      >
        ⚙
      </button>
    </header>

    <div class="config-content">
      <RadialMenuPreview
        :items="items"
        :selected-id="selectedId"
        @select="selectItem"
        @add="openAddForm"
      />
      <MenuItemsPanel
        :items="items"
        :selected-id="selectedId"
        @select="selectItem"
        @edit="openEditForm"
        @remove="removeItem"
      />
    </div>

    <footer class="config-footer">
      <span class="config-footer__status">
        <i :class="{ 'is-saved': status.startsWith('Salvo') }" />
        {{ status }}
      </span>
      <div>
        <button type="button" class="button-restore" @click="restoreDefaults">
          Restaurar padrão
        </button>
        <button type="button" class="button-primary" @click="saveSession">Salvar</button>
      </div>
    </footer>

    <MenuItemFormModal
      v-if="formMode"
      :item="editingItem"
      @save="saveItem"
      @cancel="closeForm"
    />

    <div
      v-if="showAppSettings"
      class="simple-modal-backdrop"
      @mousedown.self="showAppSettings = false"
    >
      <section class="simple-modal" role="dialog" aria-modal="true" aria-labelledby="app-settings-title">
        <span aria-hidden="true">⚙</span>
        <h2 id="app-settings-title">Configurações do app</h2>
        <p>Configurações do app serão implementadas na próxima etapa.</p>
        <button type="button" @click="showAppSettings = false">Entendi</button>
      </section>
    </div>
  </main>
</template>

<style scoped>
.config-window {
  display: grid;
  min-width: 760px;
  min-height: 600px;
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
  grid-template-columns: minmax(480px, 1.25fr) minmax(330px, 0.75fr);
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

.config-footer > div {
  gap: 9px;
}

.config-footer button {
  padding: 10px 17px;
  border-radius: 10px;
  cursor: pointer;
}

.button-restore {
  border: 1px solid rgb(255 255 255 / 10%);
  color: #b9bdcb;
  background: rgb(255 255 255 / 4%);
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

@media (max-width: 940px) {
  .config-content {
    grid-template-columns: minmax(430px, 1fr) minmax(300px, 0.72fr);
  }
}
</style>
