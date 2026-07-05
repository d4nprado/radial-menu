<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import menuConfigJson from '../config/menu.json'
import type { ConfigLoadResponse, MenuConfig, MenuItem } from '../types/menu'
import AppPreferencesModal from './AppPreferencesModal.vue'
import MenuItemFormModal from './MenuItemFormModal.vue'
import MenuItemsPanel from './MenuItemsPanel.vue'
import RadialMenuPreview from './RadialMenuPreview.vue'

const defaultConfig = menuConfigJson as MenuConfig
const shortcut = ref(defaultConfig.shortcut)
const items = ref<MenuItem[]>([])
const selectedId = ref<string | null>(null)
const formMode = ref<'add' | 'edit' | null>(null)
const editingItem = ref<MenuItem | null>(null)
const showPreferences = ref(false)
const busy = ref(true)
const status = ref('Carregando configuração…')
const statusKind = ref<'neutral' | 'success' | 'error'>('neutral')

function clone<T>(value: T): T {
  return structuredClone(value)
}

function setStatus(message: string, kind: 'neutral' | 'success' | 'error' = 'neutral') {
  status.value = message
  statusKind.value = kind
}

async function loadConfig() {
  busy.value = true
  try {
    const response = await invoke<ConfigLoadResponse>('load_launcher_config')
    shortcut.value = response.config.shortcut
    items.value = clone(response.config.items)
    selectedId.value = items.value[0]?.id ?? null
    setStatus(response.warning ?? 'Configuração carregada', response.warning ? 'error' : 'neutral')
  } catch (cause) {
    items.value = clone(defaultConfig.items)
    setStatus(typeof cause === 'string' ? cause : 'Não foi possível carregar. Usando o padrão.', 'error')
  } finally {
    busy.value = false
  }
}

function select(id: string) { selectedId.value = id }
function add() { editingItem.value = null; formMode.value = 'add' }
function edit(item: MenuItem) { editingItem.value = clone(item); formMode.value = 'edit' }
function closeForm() { formMode.value = null; editingItem.value = null }

function uniqueId(label: string) {
  const base = label.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase()
    .replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'acao'
  let id = base
  let suffix = 2
  while (items.value.some((item) => item.id === id)) id = `${base}-${suffix++}`
  return id
}

function applyItem(item: MenuItem) {
  if (formMode.value === 'edit') {
    const index = items.value.findIndex((current) => current.id === item.id)
    if (index >= 0) items.value[index] = item
  } else {
    item.id = uniqueId(item.label)
    items.value.push(item)
  }
  selectedId.value = item.id
  setStatus('Alterações ainda não salvas')
  closeForm()
}

function remove(id: string) {
  const index = items.value.findIndex((item) => item.id === id)
  if (index < 0) return
  items.value.splice(index, 1)
  selectedId.value = items.value[index]?.id ?? items.value[index - 1]?.id ?? null
  setStatus('Alterações ainda não salvas')
}

async function save() {
  busy.value = true
  try {
    await invoke('save_launcher_config', {
      config: { shortcut: shortcut.value, items: items.value },
    })
    setStatus('Configuração salva', 'success')
  } catch (cause) {
    setStatus(typeof cause === 'string' ? cause : 'Não foi possível salvar a configuração.', 'error')
  } finally {
    busy.value = false
  }
}

async function restore() {
  if (!window.confirm('Restaurar todos os itens para a configuração padrão?')) return
  busy.value = true
  try {
    const config = await invoke<MenuConfig>('reset_launcher_config')
    shortcut.value = config.shortcut
    items.value = clone(config.items)
    selectedId.value = items.value[0]?.id ?? null
    setStatus('Configuração padrão restaurada', 'success')
  } catch (cause) {
    setStatus(typeof cause === 'string' ? cause : 'Não foi possível restaurar o padrão.', 'error')
  } finally {
    busy.value = false
  }
}

onMounted(loadConfig)
</script>

<template>
  <main class="config">
    <header class="top">
      <div class="brand"><span>O</span><div><h1>Orbit Launcher</h1><p>Configure os atalhos do menu radial</p></div></div>
      <button class="gear" title="Configurações do app" aria-label="Configurações do app" @click="showPreferences = true">⚙</button>
    </header>
    <div class="content">
      <RadialMenuPreview :items="items" :selected-id="selectedId" @select="select" @add="add" />
      <MenuItemsPanel :items="items" :selected-id="selectedId" @select="select" @edit="edit" @remove="remove" />
    </div>
    <footer class="bottom">
      <span class="status" :class="statusKind"><i />{{ status }}</span>
      <div><button :disabled="busy" @click="restore">Restaurar padrão</button><button class="primary" :disabled="busy" @click="save">Salvar</button></div>
    </footer>
    <MenuItemFormModal v-if="formMode" :item="editingItem" @save="applyItem" @cancel="closeForm" />
    <AppPreferencesModal v-if="showPreferences" @close="showPreferences = false" />
  </main>
</template>

<style scoped>
.config{display:grid;min-width:760px;min-height:600px;padding:0 34px 22px;overflow:auto;grid-template-rows:92px minmax(0,1fr) 70px;color:#f4f5fb;background:radial-gradient(circle at 12% 0%,#5d4ad429,transparent 28%),radial-gradient(circle at 95% 90%,#3d98be14,transparent 26%),#090b14}.top,.brand,.bottom,.bottom>div,.status{display:flex;align-items:center}.top{justify-content:space-between;border-bottom:1px solid #ffffff12}.brand{gap:13px}.brand>span{display:grid;width:39px;height:39px;place-items:center;border:1px solid #8b7cff73;border-radius:13px;color:#c5bfff;background:#8b7cff26;font-size:17px;font-weight:700}.brand h1{margin:0;font-size:18px}.brand p{margin:4px 0 0;color:#777c91;font-size:10px}.gear{display:grid;width:38px;height:38px;padding:0;place-items:center;border:1px solid #ffffff17;border-radius:11px;color:#a7aabc;background:#ffffff0a;font-size:17px;cursor:pointer}.gear:hover{color:#d2ceff;border-color:#8b7cff4d}.content{display:grid;min-height:0;padding:24px 0;grid-template-columns:minmax(480px,1.25fr) minmax(330px,.75fr);gap:20px}.bottom{justify-content:space-between;border-top:1px solid #ffffff12}.status{color:#777c91;font-size:10px;gap:8px}.status i{width:6px;height:6px;border-radius:50%;background:#f0b85e}.status.success{color:#86daca}.status.success i{background:#55d6be;box-shadow:0 0 8px #55d6be99}.status.error{color:#ff9bae}.status.error i{background:#ff647e}.bottom>div{gap:9px}.bottom button{padding:10px 17px;border:1px solid #ffffff1a;border-radius:10px;color:#b9bdcb;background:#ffffff0a;cursor:pointer}.bottom .primary{min-width:86px;border-color:#7669e2;color:#fff;background:linear-gradient(135deg,#796bea,#5c4fc8)}button:disabled{cursor:not-allowed;opacity:.5}
</style>
