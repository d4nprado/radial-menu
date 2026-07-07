<script setup lang="ts">
import { computed, reactive, ref, toRaw, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  MenuAction,
  MenuItem,
  SystemActionTarget,
} from '../types/menu'

const props = defineProps<{
  item?: MenuItem | null
  allowGroups: boolean
}>()

const emit = defineEmits<{
  save: [item: MenuItem]
  cancel: []
}>()

type ActionType = Exclude<MenuAction['type'], 'group'>
type ItemKind = 'action' | 'group'
type StreamControl = 'scene' | 'recording' | 'streaming' | 'audio' | 'source'
type StreamActionKey =
  | 'set_scene'
  | 'start_recording'
  | 'stop_recording'
  | 'toggle_recording'
  | 'start_streaming'
  | 'stop_streaming'
  | 'toggle_streaming'
  | 'mute_input'
  | 'unmute_input'
  | 'toggle_input_mute'
  | 'show_source'
  | 'hide_source'
  | 'toggle_source_visibility'

type StreamControlOption = {
  key: StreamControl
  label: string
  description: string
}

const streamControls: StreamControlOption[] = [
  { key: 'scene', label: 'Cena', description: 'Trocar cena do OBS' },
  { key: 'recording', label: 'Gravação', description: 'Iniciar/parar gravação' },
  { key: 'streaming', label: 'Transmissão', description: 'Iniciar/parar live' },
  { key: 'audio', label: 'Áudio', description: 'Mutar/desmutar input' },
  { key: 'source', label: 'Fonte', description: 'Mostrar/ocultar fonte' },
]

type StreamActionOption = {
  key: StreamActionKey
  label: string
}

const streamActionOptions: Record<StreamControl, StreamActionOption[]> = {
  scene: [
    { key: 'set_scene', label: 'Trocar cena' },
  ],
  recording: [
    { key: 'start_recording', label: 'Iniciar gravação' },
    { key: 'stop_recording', label: 'Parar gravação' },
    { key: 'toggle_recording', label: 'Alternar gravação' },
  ],
  streaming: [
    { key: 'start_streaming', label: 'Iniciar transmissão' },
    { key: 'stop_streaming', label: 'Parar transmissão' },
    { key: 'toggle_streaming', label: 'Alternar transmissão' },
  ],
  audio: [
    { key: 'mute_input', label: 'Mutar input de áudio' },
    { key: 'unmute_input', label: 'Desmutar input de áudio' },
    { key: 'toggle_input_mute', label: 'Alternar mute de input de áudio' },
  ],
  source: [
    { key: 'show_source', label: 'Mostrar fonte' },
    { key: 'hide_source', label: 'Ocultar fonte' },
    { key: 'toggle_source_visibility', label: 'Alternar visibilidade de fonte' },
  ],
}

const defaultStreamActionKeys: Record<StreamControl, StreamActionKey> = {
  scene: 'set_scene',
  recording: 'toggle_recording',
  streaming: 'toggle_streaming',
  audio: 'toggle_input_mute',
  source: 'toggle_source_visibility',
}

type FormState = {
  id: string
  label: string
  hint: string
  icon: string
  accent: string
  itemKind: ItemKind
  actionType: ActionType
  value: string
  systemTarget: SystemActionTarget
  streamControl: StreamControl
  streamActionKey: StreamActionKey
  streamSceneName: string
  streamInputName: string
  streamSourceName: string
}

const form = reactive<FormState>({
  id: '',
  label: '',
  hint: '',
  icon: '',
  accent: '#8b7cff',
  itemKind: 'action',
  actionType: 'program',
  value: '',
  systemTarget: 'explorer',
  streamControl: 'scene',
  streamActionKey: 'set_scene',
  streamSceneName: '',
  streamInputName: '',
  streamSourceName: '',
})
const obsScenes = ref<string[]>([])
const obsInputs = ref<string[]>([])
const obsSources = ref<string[]>([])
const streamStatus = ref('')
const isLoadingScenes = ref(false)
const isLoadingInputs = ref(false)
const isLoadingSources = ref(false)

const isEditing = computed(() => Boolean(props.item))
const isGroup = computed(() => form.itemKind === 'group')
const needsStreamScene = computed(() =>
  form.streamControl === 'scene' || form.streamControl === 'source',
)
const needsStreamInput = computed(() => form.streamControl === 'audio')
const needsStreamSource = computed(() => form.streamControl === 'source')
const currentStreamActionOptions = computed(() => streamActionOptions[form.streamControl])
const streamFieldsValid = computed(() =>
  (!needsStreamScene.value || form.streamSceneName.trim())
  && (!needsStreamInput.value || form.streamInputName.trim())
  && (!needsStreamSource.value || form.streamSourceName.trim()),
)
const requiresValue = computed(() =>
  !isGroup.value && form.actionType !== 'system' && form.actionType !== 'stream',
)
const canSave = computed(() =>
  form.label.trim()
  && form.hint.trim()
  && form.icon.trim()
  && (!requiresValue.value || form.value.trim())
  && (form.actionType !== 'stream' || streamFieldsValid.value),
)
const savedStreamSceneMissing = computed(() =>
  form.actionType === 'stream'
  && Boolean(form.streamSceneName.trim())
  && !obsScenes.value.includes(form.streamSceneName),
)
const savedStreamInputMissing = computed(() =>
  form.actionType === 'stream'
  && Boolean(form.streamInputName.trim())
  && !obsInputs.value.includes(form.streamInputName),
)
const savedStreamSourceMissing = computed(() =>
  form.actionType === 'stream'
  && Boolean(form.streamSourceName.trim())
  && !obsSources.value.includes(form.streamSourceName),
)

watch(
  () => props.item,
  (item) => {
    form.id = item?.id ?? ''
    form.label = item?.label ?? ''
    form.hint = item?.hint ?? ''
    form.icon = item?.icon ?? ''
    form.accent = item?.accent ?? '#8b7cff'
    form.itemKind = item?.action.type === 'group' ? 'group' : 'action'
    form.actionType = item && item.action.type !== 'group'
      ? item.action.type
      : 'program'
    form.systemTarget = item?.action.type === 'system' ? item.action.target : 'explorer'
    form.value = getActionValue(item?.action)
    form.streamControl = getStreamControl(item?.action)
    form.streamActionKey = getStreamActionKey(item?.action)
    form.streamSceneName = item?.action.type === 'stream' ? item.action.sceneName ?? '' : ''
    form.streamInputName = item?.action.type === 'stream' ? item.action.inputName ?? '' : ''
    form.streamSourceName = item?.action.type === 'stream' ? item.action.sourceName ?? '' : ''
    streamStatus.value = ''
  },
  { immediate: true },
)

function getActionValue(action?: MenuAction) {
  if (!action || action.type === 'system' || action.type === 'group') return ''
  if (action.type === 'stream') return ''
  return action.type === 'url' ? action.url : action.path
}

function handleStreamSceneChange() {
  obsSources.value = []
  form.streamSourceName = ''
}

function selectStreamControl(control: StreamControl) {
  form.streamControl = control
  form.streamActionKey = defaultStreamActionKeys[control]
  streamStatus.value = ''
}

function getStreamControl(action?: MenuAction): StreamControl {
  if (!action || action.type !== 'stream') return 'scene'
  if (action.operation === 'set_scene') return 'scene'
  if (
    action.operation === 'toggle_recording'
    || action.operation === 'start_recording'
    || action.operation === 'stop_recording'
  ) return 'recording'
  if (
    action.operation === 'toggle_streaming'
    || action.operation === 'start_streaming'
    || action.operation === 'stop_streaming'
  ) return 'streaming'
  if (action.operation === 'toggle_input_mute' || action.operation === 'set_input_mute') {
    return 'audio'
  }
  return 'source'
}

function getStreamActionKey(action?: MenuAction): StreamActionKey {
  if (!action || action.type !== 'stream') return 'set_scene'
  if (action.operation === 'set_scene') return 'set_scene'
  if (action.operation === 'start_recording') return 'start_recording'
  if (action.operation === 'stop_recording') return 'stop_recording'
  if (action.operation === 'toggle_recording') return 'toggle_recording'
  if (action.operation === 'start_streaming') return 'start_streaming'
  if (action.operation === 'stop_streaming') return 'stop_streaming'
  if (action.operation === 'toggle_streaming') return 'toggle_streaming'
  if (action.operation === 'set_input_mute') {
    return action.muted === false ? 'unmute_input' : 'mute_input'
  }
  if (action.operation === 'toggle_input_mute') return 'toggle_input_mute'
  if (action.operation === 'set_source_visibility') {
    return action.visible === false ? 'hide_source' : 'show_source'
  }
  return 'toggle_source_visibility'
}

async function selectProgram() {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Programas', extensions: ['exe', 'com', 'bat', 'cmd'] }],
  })
  if (typeof selected === 'string') form.value = selected
}

async function selectDirectory() {
  const selected = await open({ multiple: false, directory: true })
  if (typeof selected === 'string') form.value = selected
}

async function loadObsScenes() {
  isLoadingScenes.value = true
  streamStatus.value = ''
  try {
    obsScenes.value = await invoke<string[]>('list_obs_scenes')
    if (!obsScenes.value.length) {
      streamStatus.value = 'Nenhuma cena retornada pelo OBS.'
      return
    }
    if (!form.streamSceneName.trim()) form.streamSceneName = obsScenes.value[0] ?? ''
    streamStatus.value = 'Cenas carregadas do OBS.'
  } catch (cause) {
    streamStatus.value = typeof cause === 'string'
      ? cause
      : 'Não foi possível carregar as cenas do OBS.'
  } finally {
    isLoadingScenes.value = false
  }
}

async function loadObsInputs() {
  isLoadingInputs.value = true
  streamStatus.value = ''
  try {
    obsInputs.value = await invoke<string[]>('list_obs_inputs')
    if (!obsInputs.value.length) {
      streamStatus.value = 'Nenhum input retornado pelo OBS.'
      return
    }
    if (!form.streamInputName.trim()) form.streamInputName = obsInputs.value[0] ?? ''
    streamStatus.value = 'Inputs carregados do OBS.'
  } catch (cause) {
    streamStatus.value = typeof cause === 'string'
      ? cause
      : 'Não foi possível carregar os inputs do OBS.'
  } finally {
    isLoadingInputs.value = false
  }
}

async function loadObsSources() {
  if (!form.streamSceneName.trim()) {
    streamStatus.value = 'Escolha uma cena antes de carregar as fontes.'
    return
  }

  isLoadingSources.value = true
  streamStatus.value = ''
  try {
    obsSources.value = await invoke<string[]>('list_obs_sources_for_scene', {
      sceneName: form.streamSceneName,
    })
    if (!obsSources.value.length) {
      streamStatus.value = 'Nenhuma fonte retornada pelo OBS para esta cena.'
      return
    }
    if (!form.streamSourceName.trim()) form.streamSourceName = obsSources.value[0] ?? ''
    streamStatus.value = 'Fontes carregadas do OBS.'
  } catch (cause) {
    streamStatus.value = typeof cause === 'string'
      ? cause
      : 'Não foi possível carregar as fontes do OBS.'
  } finally {
    isLoadingSources.value = false
  }
}

function buildAction(): MenuAction {
  if (isGroup.value) {
    const items = props.item?.action.type === 'group'
      ? structuredClone(toRaw(props.item.action.items))
      : []
    return { type: 'group', items }
  }
  if (form.actionType === 'program') return { type: 'program', path: form.value.trim() }
  if (form.actionType === 'directory') return { type: 'directory', path: form.value.trim() }
  if (form.actionType === 'url') {
    const value = form.value.trim()
    const url = /^https?:\/\//i.test(value) || value.includes('://')
      ? value
      : `https://${value}`
    return { type: 'url', url }
  }
  if (form.actionType === 'stream') {
    const action: MenuAction = {
      type: 'stream',
      provider: 'obs',
      operation: 'set_scene',
    }

    if (
      form.streamActionKey === 'start_recording'
      || form.streamActionKey === 'stop_recording'
      || form.streamActionKey === 'toggle_recording'
      || form.streamActionKey === 'start_streaming'
      || form.streamActionKey === 'stop_streaming'
      || form.streamActionKey === 'toggle_streaming'
      || form.streamActionKey === 'toggle_input_mute'
      || form.streamActionKey === 'toggle_source_visibility'
    ) {
      action.operation = form.streamActionKey
    }

    if (form.streamControl === 'audio') {
      if (form.streamActionKey === 'mute_input' || form.streamActionKey === 'unmute_input') {
        action.operation = 'set_input_mute'
        action.muted = form.streamActionKey === 'mute_input'
      }
      action.inputName = form.streamInputName.trim()
    }

    if (form.streamControl === 'source') {
      if (form.streamActionKey === 'show_source' || form.streamActionKey === 'hide_source') {
        action.operation = 'set_source_visibility'
        action.visible = form.streamActionKey === 'show_source'
      }
      action.sceneName = form.streamSceneName.trim()
      action.sourceName = form.streamSourceName.trim()
    }

    if (form.streamControl === 'scene') action.sceneName = form.streamSceneName.trim()

    return action
  }
  return { type: 'system', target: form.systemTarget }
}

function selectItemKind(kind: ItemKind) {
  if (!props.allowGroups || isEditing.value) return
  form.itemKind = kind
}

function submit() {
  if (!canSave.value) return

  emit('save', {
    id: form.id,
    label: form.label.trim(),
    hint: form.hint.trim(),
    icon: form.icon.trim(),
    accent: form.accent,
    action: buildAction(),
  })
}
</script>

<template>
  <div class="modal-backdrop" role="presentation" @mousedown.self="emit('cancel')">
    <form class="item-modal" @submit.prevent="submit">
      <header>
        <div>
          <span>{{ isEditing ? (isGroup ? 'EDITAR GRUPO' : 'EDITAR AÇÃO') : (isGroup ? 'NOVO GRUPO' : 'NOVA AÇÃO') }}</span>
          <h2>{{ isEditing ? (isGroup ? 'Editar grupo' : 'Editar ação') : 'Adicionar ao menu radial' }}</h2>
        </div>
        <button type="button" class="item-modal__close" aria-label="Fechar" @click="emit('cancel')">
          ×
        </button>
      </header>

      <div class="item-modal__grid">
        <label>
          <span>Label</span>
          <input v-model="form.label" required placeholder="Ex.: Visual Studio Code">
        </label>

        <label>
          <span>Hint</span>
          <input v-model="form.hint" required placeholder="Ex.: Desenvolvimento">
        </label>

        <label>
          <span>Ícone / texto curto</span>
          <input v-model="form.icon" required maxlength="3" placeholder="VS">
        </label>

        <label>
          <span>Cor / accent</span>
          <span class="color-field">
            <input v-model="form.accent" type="color" aria-label="Escolher cor">
            <input v-model="form.accent" required pattern="#[0-9a-fA-F]{6}" aria-label="Código da cor">
          </span>
        </label>

        <div v-if="allowGroups" class="item-modal__wide item-kind-field">
          <span>Categoria do item</span>
          <div class="item-kind-toggle" role="group" aria-label="Categoria do item">
            <button
              type="button"
              :class="{ 'is-active': !isGroup }"
              :aria-pressed="!isGroup"
              :disabled="isEditing"
              @click="selectItemKind('action')"
            >
              <strong>Ação</strong>
              <small>Executa um comando</small>
            </button>
            <button
              type="button"
              :class="{ 'is-active': isGroup }"
              :aria-pressed="isGroup"
              :disabled="isEditing"
              @click="selectItemKind('group')"
            >
              <strong>Grupo</strong>
              <small>Organiza outras ações</small>
            </button>
          </div>
          <small v-if="isEditing" class="item-kind-field__hint">
            A categoria não pode ser alterada durante a edição.
          </small>
        </div>

        <label v-if="!isGroup" class="item-modal__wide">
          <span>Tipo de ação</span>
          <select v-model="form.actionType">
            <option value="program">Abrir programa</option>
            <option value="directory">Abrir diretório</option>
            <option value="url">Abrir URL</option>
            <option value="system">Ação padrão do sistema</option>
            <option value="stream">Stream</option>
          </select>
        </label>

        <label v-if="!isGroup && form.actionType === 'system'" class="item-modal__wide">
          <span>Ação padrão</span>
          <select v-model="form.systemTarget">
            <option value="explorer">Explorador de arquivos</option>
            <option value="terminal">Terminal</option>
            <option value="calculator">Calculadora</option>
            <option value="notepad">Bloco de notas</option>
          </select>
        </label>

        <div v-else-if="!isGroup && form.actionType === 'stream'" class="item-modal__wide stream-fields">
          <label>
            <span>Plataforma</span>
            <select disabled>
              <option>OBS Studio</option>
            </select>
          </label>

          <div class="stream-fields__wide stream-control-field">
            <span>Controle</span>
            <div class="stream-control-grid" role="group" aria-label="Controle OBS">
              <button
                v-for="control in streamControls"
                :key="control.key"
                type="button"
                :class="{ 'is-active': form.streamControl === control.key }"
                :aria-pressed="form.streamControl === control.key"
                @click="selectStreamControl(control.key)"
              >
                <strong>{{ control.label }}</strong>
                <small>{{ control.description }}</small>
              </button>
            </div>
          </div>

          <label>
            <span>Ação</span>
            <select v-model="form.streamActionKey">
              <option
                v-for="actionOption in currentStreamActionOptions"
                :key="actionOption.key"
                :value="actionOption.key"
              >
                {{ actionOption.label }}
              </option>
            </select>
          </label>

          <label v-if="needsStreamScene" class="stream-fields__wide">
            <span>Cena do OBS</span>
            <span class="path-field">
              <select
                v-model="form.streamSceneName"
                required
                @change="handleStreamSceneChange"
              >
                <option value="" disabled>Selecione uma cena</option>
                <option
                  v-if="savedStreamSceneMissing"
                  :value="form.streamSceneName"
                >
                  Cena atual: {{ form.streamSceneName }}
                </option>
                <option
                  v-for="scene in obsScenes"
                  :key="scene"
                  :value="scene"
                >
                  {{ scene }}
                </option>
              </select>
              <button
                type="button"
                :disabled="isLoadingScenes"
                @click="loadObsScenes"
              >
                {{ isLoadingScenes ? 'Carregando...' : 'Carregar cenas' }}
              </button>
            </span>
          </label>

          <label v-if="needsStreamInput" class="stream-fields__wide">
            <span>Input de áudio</span>
            <span class="path-field">
              <select v-model="form.streamInputName" required>
                <option value="" disabled>Selecione um input</option>
                <option
                  v-if="savedStreamInputMissing"
                  :value="form.streamInputName"
                >
                  Input atual: {{ form.streamInputName }}
                </option>
                <option
                  v-for="input in obsInputs"
                  :key="input"
                  :value="input"
                >
                  {{ input }}
                </option>
              </select>
              <button
                type="button"
                :disabled="isLoadingInputs"
                @click="loadObsInputs"
              >
                {{ isLoadingInputs ? 'Carregando...' : 'Carregar inputs de áudio' }}
              </button>
            </span>
          </label>

          <label v-if="needsStreamSource" class="stream-fields__wide">
            <span>Fonte</span>
            <span class="path-field">
              <select v-model="form.streamSourceName" required>
                <option value="" disabled>Selecione uma fonte</option>
                <option
                  v-if="savedStreamSourceMissing"
                  :value="form.streamSourceName"
                >
                  Fonte atual: {{ form.streamSourceName }}
                </option>
                <option
                  v-for="source in obsSources"
                  :key="source"
                  :value="source"
                >
                  {{ source }}
                </option>
              </select>
              <button
                type="button"
                :disabled="isLoadingSources || !form.streamSceneName.trim()"
                @click="loadObsSources"
              >
                {{ isLoadingSources ? 'Carregando...' : 'Carregar fontes' }}
              </button>
            </span>
          </label>

          <p v-if="streamStatus" class="stream-fields__status">{{ streamStatus }}</p>
        </div>

        <div v-else-if="isGroup" class="item-modal__wide group-note">
          Os itens poderão ser adicionados depois, ao abrir o grupo na configuração.
        </div>

        <label v-else class="item-modal__wide">
          <span>
            {{ form.actionType === 'url' ? 'URL' : 'Caminho' }}
          </span>
          <span class="path-field">
            <input
              v-model="form.value"
              required
              type="text"
              :inputmode="form.actionType === 'url' ? 'url' : 'text'"
              :placeholder="form.actionType === 'url' ? 'Ex.: https://github.com, youtube.com, chatgpt.com' : 'C:\\Caminho'"
            >
            <button
              v-if="form.actionType === 'program'"
              type="button"
              @click="selectProgram"
            >
              Selecionar programa
            </button>
            <button
              v-if="form.actionType === 'directory'"
              type="button"
              @click="selectDirectory"
            >
              Selecionar pasta
            </button>
          </span>
        </label>
      </div>

      <footer>
        <button type="button" class="button-cancel" @click="emit('cancel')">Cancelar</button>
        <button type="submit" class="button-save" :disabled="!canSave">
          {{ isEditing ? 'Salvar alterações' : (isGroup ? 'Adicionar grupo' : 'Adicionar ação') }}
        </button>
      </footer>
    </form>
  </div>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  z-index: 30;
  inset: 0;
  display: grid;
  padding: 30px;
  place-items: center;
  background: rgb(4 6 12 / 72%);
  backdrop-filter: blur(8px);
}

.item-modal {
  width: min(100%, 650px);
  max-height: 100%;
  padding: 26px;
  overflow-y: auto;
  border: 1px solid rgb(255 255 255 / 11%);
  border-radius: 20px;
  color: #f2f3fa;
  background: #151827;
  box-shadow: 0 30px 90px rgb(0 0 0 / 52%);
}

header,
footer,
.color-field,
.path-field {
  display: flex;
}

header {
  align-items: flex-start;
  justify-content: space-between;
}

header span {
  color: #8d82ff;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.14em;
}

h2 {
  margin: 5px 0 0;
  font-size: 21px;
}

.item-modal__close {
  width: 32px;
  height: 32px;
  border: 0;
  border-radius: 9px;
  color: #9297aa;
  background: rgb(255 255 255 / 5%);
  font-size: 20px;
  cursor: pointer;
}

.item-modal__grid {
  display: grid;
  margin-top: 24px;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

label {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 7px;
}

label > span:first-child {
  color: #a5a9ba;
  font-size: 10px;
  font-weight: 600;
}

.item-modal__wide {
  grid-column: 1 / -1;
}

.item-kind-field {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.item-kind-field > span {
  color: #a5a9ba;
  font-size: 10px;
  font-weight: 600;
}

.item-kind-toggle {
  display: grid;
  padding: 4px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 11px;
  background: #0e111d;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
}

.item-kind-toggle button {
  display: flex;
  min-height: 48px;
  padding: 8px 12px;
  align-items: flex-start;
  justify-content: center;
  flex-direction: column;
  border: 1px solid transparent;
  border-radius: 8px;
  color: #858a9e;
  text-align: left;
  background: transparent;
  cursor: pointer;
  gap: 3px;
}

.item-kind-toggle button:hover:not(:disabled) {
  color: #cbc7ec;
  background: rgb(139 124 255 / 6%);
}

.item-kind-toggle button.is-active {
  border-color: rgb(139 124 255 / 45%);
  color: #e7e5fa;
  background: linear-gradient(135deg, rgb(121 107 234 / 20%), rgb(95 81 208 / 10%));
  box-shadow: 0 4px 14px rgb(38 31 91 / 18%);
}

.item-kind-toggle strong {
  font-size: 11px;
}

.item-kind-toggle small {
  color: #6f7488;
  font-size: 8px;
}

.item-kind-toggle button.is-active small {
  color: #9791c8;
}

.item-kind-toggle button:disabled {
  cursor: default;
}

.item-kind-field__hint {
  color: #696e82;
  font-size: 8px;
}

.group-note {
  padding: 12px 14px;
  border: 1px solid rgb(139 124 255 / 18%);
  border-radius: 9px;
  color: #8f94aa;
  background: rgb(139 124 255 / 6%);
  font-size: 10px;
  line-height: 1.5;
}

.stream-fields {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

.stream-control-field {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.stream-control-field > span {
  color: #a5a9ba;
  font-size: 10px;
  font-weight: 600;
}

.stream-control-grid {
  display: grid;
  padding: 4px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 11px;
  background: #0e111d;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 4px;
}

.stream-control-grid button {
  display: flex;
  min-width: 0;
  min-height: 58px;
  padding: 8px 9px;
  align-items: flex-start;
  justify-content: center;
  flex-direction: column;
  border: 1px solid transparent;
  border-radius: 8px;
  color: #858a9e;
  text-align: left;
  background: transparent;
  cursor: pointer;
  gap: 4px;
}

.stream-control-grid button:hover {
  color: #cbc7ec;
  background: rgb(139 124 255 / 6%);
}

.stream-control-grid button.is-active {
  border-color: rgb(139 124 255 / 45%);
  color: #e7e5fa;
  background: linear-gradient(135deg, rgb(121 107 234 / 20%), rgb(95 81 208 / 10%));
  box-shadow: 0 4px 14px rgb(38 31 91 / 18%);
}

.stream-control-grid strong,
.stream-control-grid small {
  overflow: hidden;
  max-width: 100%;
  text-overflow: ellipsis;
}

.stream-control-grid strong {
  font-size: 10px;
}

.stream-control-grid small {
  color: #6f7488;
  font-size: 8px;
  line-height: 1.25;
}

.stream-control-grid button.is-active small {
  color: #9791c8;
}

.stream-fields__wide,
.stream-fields__status {
  grid-column: 1 / -1;
}

.stream-fields__status {
  margin: 0;
  color: #8f94aa;
  font-size: 10px;
  line-height: 1.5;
}

input,
select {
  width: 100%;
  min-width: 0;
  height: 40px;
  padding: 0 12px;
  border: 1px solid rgb(255 255 255 / 10%);
  border-radius: 9px;
  outline: none;
  color: #f0f1f8;
  background: #0e111d;
}

input:focus,
select:focus {
  border-color: rgb(139 124 255 / 70%);
  box-shadow: 0 0 0 3px rgb(139 124 255 / 11%);
}

select:disabled {
  cursor: not-allowed;
  opacity: 0.65;
}

input[type="color"] {
  width: 46px;
  padding: 5px;
  flex: 0 0 auto;
}

.color-field,
.path-field {
  gap: 8px;
}

.path-field button {
  padding: 0 13px;
  flex: 0 0 auto;
  border: 1px solid rgb(139 124 255 / 25%);
  border-radius: 9px;
  color: #bcb6ff;
  background: rgb(139 124 255 / 9%);
  cursor: pointer;
}

footer {
  margin-top: 25px;
  padding-top: 18px;
  justify-content: flex-end;
  border-top: 1px solid rgb(255 255 255 / 7%);
  gap: 9px;
}

footer button {
  padding: 10px 16px;
  border-radius: 9px;
  cursor: pointer;
}

.button-cancel {
  border: 1px solid rgb(255 255 255 / 10%);
  color: #bfc2d0;
  background: transparent;
}

.button-save {
  border: 1px solid #7567e8;
  color: white;
  background: linear-gradient(135deg, #796bea, #5f51d0);
}

.button-save:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}
</style>
