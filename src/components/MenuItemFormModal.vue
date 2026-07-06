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
})
const obsScenes = ref<string[]>([])
const streamStatus = ref('')
const isLoadingScenes = ref(false)

const isEditing = computed(() => Boolean(props.item))
const isGroup = computed(() => form.itemKind === 'group')
const requiresValue = computed(() =>
  !isGroup.value && form.actionType !== 'system',
)
const canSave = computed(() =>
  form.label.trim()
  && form.hint.trim()
  && form.icon.trim()
  && (!requiresValue.value || form.value.trim()),
)
const savedStreamSceneMissing = computed(() =>
  form.actionType === 'stream'
  && Boolean(form.value.trim())
  && !obsScenes.value.includes(form.value),
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
  },
  { immediate: true },
)

function getActionValue(action?: MenuAction) {
  if (!action || action.type === 'system' || action.type === 'group') return ''
  if (action.type === 'stream') return action.sceneName
  return action.type === 'url' ? action.url : action.path
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
    if (!form.value.trim()) form.value = obsScenes.value[0] ?? ''
    streamStatus.value = 'Cenas carregadas do OBS.'
  } catch (cause) {
    streamStatus.value = typeof cause === 'string'
      ? cause
      : 'Não foi possível carregar as cenas do OBS.'
  } finally {
    isLoadingScenes.value = false
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
    return {
      type: 'stream',
      provider: 'obs',
      operation: 'set_scene',
      sceneName: form.value.trim(),
    }
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

          <label>
            <span>Comando</span>
            <select disabled>
              <option>Trocar cena</option>
            </select>
          </label>

          <label class="stream-fields__wide">
            <span>Cena do OBS</span>
            <span class="path-field">
              <select v-model="form.value" required>
                <option value="" disabled>Selecione uma cena</option>
                <option
                  v-if="savedStreamSceneMissing"
                  :value="form.value"
                >
                  Cena atual: {{ form.value }}
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
