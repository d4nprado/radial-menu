<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'
import type { AppPreferences, PreferencesLoadResponse } from '../types/menu'

const DEFAULT_SHORTCUT = 'Ctrl+Space'
const emit = defineEmits<{ close: [] }>()

function defaultPreferences(): AppPreferences {
  return {
    startWithWindows: false,
    openMenuShortcut: { type: 'keyboard', value: DEFAULT_SHORTCUT },
    futureMouseShortcut: { enabled: false, button: null },
  }
}

const preferences = ref<AppPreferences>(defaultPreferences())
const capturedShortcut = ref(DEFAULT_SHORTCUT)
const isCapturing = ref(false)
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
    capturedShortcut.value = response.preferences.openMenuShortcut.value
    configPath.value = path
    if (response.warning) showMessage(response.warning, true)
  } catch (cause) {
    showMessage(
      typeof cause === 'string' ? cause : 'Não foi possível carregar as preferências.',
      true,
    )
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
    showMessage(
      enabled
        ? 'Inicialização com Windows ativada.'
        : 'Inicialização com Windows desativada.',
    )
  } catch (cause) {
    preferences.value.startWithWindows = !enabled
    showMessage(
      typeof cause === 'string'
        ? cause
        : 'Não foi possível alterar a inicialização.',
      !enabled,
    )
  } finally {
    busy.value = false
  }
}

function captureShortcut(event: KeyboardEvent) {
  const ignored = ['Control', 'Shift', 'Alt', 'Meta', 'AltGraph']
  if (ignored.includes(event.key)) {
    isCapturing.value = true
    return
  }

  let key = event.key
  if (key === ' ') key = 'Space'
  else if (key.length === 1) key = key.toUpperCase()
  else if (!/^F([1-9]|1[0-9]|2[0-4])$/.test(key)
    && !['Enter', 'Escape', 'Tab', 'Backspace', 'Delete'].includes(key)) {
    showMessage(`A tecla “${event.key}” não é suportada como atalho.`, true)
    return
  }

  const parts: string[] = []
  if (event.ctrlKey) parts.push('Ctrl')
  if (event.shiftKey) parts.push('Shift')
  if (event.altKey) parts.push('Alt')
  if (event.metaKey) parts.push('Super')
  parts.push(key)

  capturedShortcut.value = parts.join('+')
  isCapturing.value = false
  showMessage('Novo atalho capturado. Clique em “Salvar atalho” para aplicar.')
}

async function saveShortcut() {
  busy.value = true
  const previous = preferences.value.openMenuShortcut.value
  const nextPreferences: AppPreferences = {
    ...preferences.value,
    openMenuShortcut: {
      type: 'keyboard',
      value: capturedShortcut.value,
    },
  }

  try {
    await invoke('save_app_preferences', { preferences: nextPreferences })
    preferences.value = nextPreferences
    showMessage(`Atalho ${capturedShortcut.value} aplicado.`)
  } catch (cause) {
    capturedShortcut.value = previous
    showMessage(
      typeof cause === 'string' ? cause : 'Não foi possível aplicar o atalho.',
      true,
    )
  } finally {
    busy.value = false
  }
}

async function openFolder() {
  try {
    await invoke('open_config_directory')
  } catch (cause) {
    showMessage(
      typeof cause === 'string' ? cause : 'Não foi possível abrir a pasta.',
      true,
    )
  }
}

async function restoreDefaults() {
  busy.value = true
  const defaults = defaultPreferences()

  try {
    await invoke('save_app_preferences', { preferences: defaults })
    preferences.value = defaults
    capturedShortcut.value = DEFAULT_SHORTCUT
    showMessage('Preferências restauradas')

    try {
      await invoke('set_autostart_enabled', { enabled: false })
    } catch (cause) {
      showMessage(
        typeof cause === 'string'
          ? `Preferências restauradas. Não foi possível confirmar o autostart: ${cause}`
          : 'Preferências restauradas. Não foi possível confirmar o autostart.',
      )
    }
  } catch (cause) {
    showMessage(
      typeof cause === 'string'
        ? cause
        : 'Não foi possível restaurar as preferências.',
      true,
    )
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
        <div>
          <small>PREFERÊNCIAS</small>
          <h2 id="preferences-title">Configurações do app</h2>
        </div>
        <button aria-label="Fechar" @click="emit('close')">×</button>
      </header>

      <div class="options" :class="{ disabled: busy }">
        <label class="option">
          <span>
            <strong>Iniciar com Windows</strong>
            <small>Executa o Orbit Launcher ao entrar no sistema.</small>
          </span>
          <input
            :checked="preferences.startWithWindows"
            type="checkbox"
            role="switch"
            :disabled="busy"
            @change="updateAutostart"
          >
        </label>

        <section class="shortcut-section">
          <div class="section-heading">
            <span>ATALHO DO LAUNCHER</span>
            <b>Teclado</b>
          </div>
          <button
            type="button"
            class="shortcut-capture"
            :class="{ capturing: isCapturing }"
            :disabled="busy"
            @focus="isCapturing = true"
            @blur="isCapturing = false"
            @keydown.prevent.stop="captureShortcut"
          >
            <small>{{ isCapturing ? 'Pressione uma combinação…' : 'Clique e pressione uma combinação' }}</small>
            <kbd>{{ capturedShortcut }}</kbd>
          </button>
          <button
            type="button"
            class="save-shortcut"
            :disabled="busy || capturedShortcut === preferences.openMenuShortcut.value"
            @click="saveShortcut"
          >
            Salvar atalho
          </button>
          <p>Suporte a Mouse 4/5 será adicionado em uma próxima etapa.</p>
        </section>

        <div class="path">
          <span>Arquivo de configuração</span>
          <code :title="configPath">{{ configPath }}</code>
          <button :disabled="busy" @click="openFolder">Abrir pasta de configuração</button>
        </div>
      </div>

      <p v-if="message" class="message" :class="{ error: isError }">{{ message }}</p>

      <footer>
        <button :disabled="busy" @click="restoreDefaults">
          Restaurar preferências padrão
        </button>
        <button class="primary" @click="emit('close')">Fechar</button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.backdrop{position:fixed;z-index:40;inset:0;display:grid;padding:30px;place-items:center;background:#03050bb8;backdrop-filter:blur(8px)}.modal{width:min(100%,590px);max-height:100%;padding:27px;overflow-y:auto;border:1px solid #ffffff1c;border-radius:21px;background:#151827;box-shadow:0 30px 90px #0008}header,footer,.option,.section-heading{display:flex;align-items:center}header{justify-content:space-between}header small{color:#8d82ff;font-size:10px;font-weight:700;letter-spacing:.14em}h2{margin:5px 0 0;font-size:21px}header button{width:32px;height:32px;border:0;border-radius:9px;color:#9297aa;background:#ffffff0d;font-size:20px;cursor:pointer}.options{margin-top:23px}.options.disabled{opacity:.72}.option{padding:16px 0;justify-content:space-between;border-bottom:1px solid #ffffff12;gap:20px}.option span{display:flex;flex-direction:column;gap:5px}.option strong{font-size:12px}.option small{color:#7f8498;font-size:10px}.option input{position:relative;width:38px;height:21px;flex:none;appearance:none;border-radius:999px;background:#34384a;cursor:pointer;transition:.15s}.option input::after{position:absolute;top:3px;left:3px;width:15px;height:15px;border-radius:50%;background:#a6aabc;content:"";transition:.15s}.option input:checked{background:#6d5edf}.option input:checked::after{left:20px;background:#fff}.shortcut-section{padding:19px 0;border-bottom:1px solid #ffffff12}.section-heading{margin-bottom:10px;justify-content:space-between}.section-heading span,.path>span{color:#a5a9ba;font-size:10px;font-weight:700;letter-spacing:.08em}.section-heading b{padding:4px 7px;border-radius:6px;color:#aaa4ee;background:#8b7cff14;font-size:9px}.shortcut-capture{display:flex;width:100%;min-height:60px;padding:10px 13px;align-items:center;justify-content:space-between;border:1px solid #ffffff1a;border-radius:10px;color:#eee;background:#0e111d;cursor:pointer;gap:12px}.shortcut-capture.capturing{border-color:#8b7cff99;box-shadow:0 0 0 3px #8b7cff1c}.shortcut-capture small{color:#777c91;font-size:9px}.shortcut-capture kbd{padding:7px 9px;border:1px solid #ffffff1a;border-radius:7px;color:#d3cfff;background:#ffffff0a;font-family:inherit;font-size:11px}.save-shortcut{margin-top:9px;padding:8px 11px;border:1px solid #8b7cff3d;border-radius:8px;color:#bcb6ff;background:#8b7cff14;cursor:pointer}.shortcut-section p{margin:11px 0 0;color:#676c80;font-size:9px}.path{display:flex;padding:18px 0 5px;flex-direction:column;gap:8px}.path code{overflow:hidden;padding:11px 12px;border:1px solid #ffffff14;border-radius:9px;color:#9297aa;background:#0e111d;font-family:Consolas,monospace;font-size:9px;text-overflow:ellipsis;white-space:nowrap}.path button{align-self:flex-start;padding:8px 11px;border:1px solid #8b7cff3d;border-radius:8px;color:#bcb6ff;background:#8b7cff14;cursor:pointer}.message{margin:15px 0 0;padding:9px 11px;border-radius:8px;color:#79dac8;background:#55d6be12;font-size:10px}.message.error{color:#ff9bae;background:#ff647e12}footer{margin-top:22px;padding-top:17px;justify-content:flex-end;border-top:1px solid #ffffff12;gap:9px}footer button{padding:9px 14px;border:1px solid #ffffff1a;border-radius:9px;color:#bfc2d0;background:transparent;cursor:pointer}.primary{border-color:#7567e8;color:#fff;background:#6759d7}button:disabled,input:disabled{cursor:not-allowed;opacity:.5}

.modal {
  max-height: calc(100vh - 60px);
  scrollbar-color: #3a3f55 transparent;
  scrollbar-width: thin;
}

.modal::-webkit-scrollbar {
  width: 8px;
}

.modal::-webkit-scrollbar-track {
  background: transparent;
}

.modal::-webkit-scrollbar-thumb {
  border: 2px solid #151827;
  border-radius: 999px;
  background: #3a3f55;
}
</style>
