<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import type { AppPreferences, PreferencesLoadResponse } from '../types/menu'

const emit = defineEmits<{ close: [] }>()
const preferences = ref<AppPreferences>({ startWithWindows: false, openConfigOnStartup: false })
const configPath = ref('Carregando…')
const busy = ref(true)
const message = ref('')
const isError = ref(false)

function showMessage(text: string, error = false) {
  message.value = text
  isError.value = error
}

async function load() {
  busy.value = true
  try {
    const [response, path] = await Promise.all([
      invoke<PreferencesLoadResponse>('get_app_preferences'),
      invoke<string>('get_config_path'),
    ])
    preferences.value = response.preferences
    configPath.value = path
    if (response.warning) showMessage(response.warning, true)
  } catch (cause) {
    showMessage(typeof cause === 'string' ? cause : 'Não foi possível carregar as preferências.', true)
  } finally {
    busy.value = false
  }
}

async function updateAutostart(event: Event) {
  const enabled = (event.target as HTMLInputElement).checked
  busy.value = true
  try {
    await invoke('set_autostart_enabled', { enabled })
    preferences.value.startWithWindows = await invoke<boolean>('get_autostart_enabled')
    showMessage(enabled ? 'Inicialização com Windows ativada.' : 'Inicialização com Windows desativada.')
  } catch (cause) {
    preferences.value.startWithWindows = !enabled
    showMessage(typeof cause === 'string' ? cause : 'Não foi possível alterar a inicialização.', true)
  } finally {
    busy.value = false
  }
}

async function savePreferences() {
  try {
    await invoke('save_app_preferences', { preferences: preferences.value })
    showMessage('Preferências salvas.')
  } catch (cause) {
    showMessage(typeof cause === 'string' ? cause : 'Não foi possível salvar as preferências.', true)
  }
}

async function openFolder() {
  try {
    await invoke('open_config_directory')
  } catch (cause) {
    showMessage(typeof cause === 'string' ? cause : 'Não foi possível abrir a pasta.', true)
  }
}

async function restoreDefaults() {
  busy.value = true
  try {
    await invoke('set_autostart_enabled', { enabled: false })
    preferences.value = { startWithWindows: false, openConfigOnStartup: false }
    await invoke('save_app_preferences', { preferences: preferences.value })
    showMessage('Preferências padrão restauradas.')
  } catch (cause) {
    showMessage(typeof cause === 'string' ? cause : 'Não foi possível restaurar as preferências.', true)
  } finally {
    busy.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="backdrop" @mousedown.self="emit('close')">
    <section class="modal" role="dialog" aria-modal="true" aria-labelledby="preferences-title">
      <header>
        <div><small>PREFERÊNCIAS</small><h2 id="preferences-title">Configurações do app</h2></div>
        <button aria-label="Fechar" @click="emit('close')">×</button>
      </header>

      <div class="options" :class="{ disabled: busy }">
        <label class="option">
          <span><strong>Iniciar com Windows</strong><small>Executa o Orbit Launcher ao entrar no sistema.</small></span>
          <input :checked="preferences.startWithWindows" type="checkbox" role="switch" :disabled="busy" @change="updateAutostart">
        </label>
        <label class="option">
          <span><strong>Abrir configuração ao iniciar</strong><small>Exibe esta janela quando o Orbit Launcher iniciar.</small></span>
          <input v-model="preferences.openConfigOnStartup" type="checkbox" role="switch" :disabled="busy" @change="savePreferences">
        </label>

        <div class="path">
          <span>Arquivo de configuração</span>
          <code :title="configPath">{{ configPath }}</code>
          <button :disabled="busy" @click="openFolder">Abrir pasta de configuração</button>
        </div>
      </div>

      <p v-if="message" class="message" :class="{ error: isError }">{{ message }}</p>

      <footer>
        <button :disabled="busy" @click="restoreDefaults">Restaurar preferências padrão</button>
        <button class="primary" @click="emit('close')">Fechar</button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.backdrop{position:fixed;z-index:40;inset:0;display:grid;padding:30px;place-items:center;background:#03050bb8;backdrop-filter:blur(8px)}.modal{width:min(100%,570px);padding:27px;border:1px solid #ffffff1c;border-radius:21px;background:#151827;box-shadow:0 30px 90px #0008}header,footer,.option{display:flex;align-items:center}header{justify-content:space-between}header small{color:#8d82ff;font-size:10px;font-weight:700;letter-spacing:.14em}h2{margin:5px 0 0;font-size:21px}header button{width:32px;height:32px;border:0;border-radius:9px;color:#9297aa;background:#ffffff0d;font-size:20px;cursor:pointer}.options{margin-top:23px}.options.disabled{opacity:.72}.option{padding:16px 0;justify-content:space-between;border-bottom:1px solid #ffffff12;gap:20px}.option span{display:flex;flex-direction:column;gap:5px}.option strong{font-size:12px}.option small{color:#7f8498;font-size:10px}.option input{position:relative;width:38px;height:21px;flex:none;appearance:none;border-radius:999px;background:#34384a;cursor:pointer;transition:.15s}.option input::after{position:absolute;top:3px;left:3px;width:15px;height:15px;border-radius:50%;background:#a6aabc;content:"";transition:.15s}.option input:checked{background:#6d5edf}.option input:checked::after{left:20px;background:#fff}.path{display:flex;padding:18px 0 5px;flex-direction:column;gap:8px}.path>span{color:#a5a9ba;font-size:10px;font-weight:600}.path code{overflow:hidden;padding:11px 12px;border:1px solid #ffffff14;border-radius:9px;color:#9297aa;background:#0e111d;font-family:Consolas,monospace;font-size:9px;text-overflow:ellipsis;white-space:nowrap}.path button{align-self:flex-start;padding:8px 11px;border:1px solid #8b7cff3d;border-radius:8px;color:#bcb6ff;background:#8b7cff14;cursor:pointer}.message{margin:15px 0 0;padding:9px 11px;border-radius:8px;color:#79dac8;background:#55d6be12;font-size:10px}.message.error{color:#ff9bae;background:#ff647e12}footer{margin-top:22px;padding-top:17px;justify-content:flex-end;border-top:1px solid #ffffff12;gap:9px}footer button{padding:9px 14px;border:1px solid #ffffff1a;border-radius:9px;color:#bfc2d0;background:transparent;cursor:pointer}.primary{border-color:#7567e8;color:#fff;background:#6759d7}button:disabled,input:disabled{cursor:not-allowed;opacity:.5}
</style>
