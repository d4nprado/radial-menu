<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  MenuAction,
  MenuItem,
  SystemActionTarget,
} from '../types/menu'

const props = defineProps<{
  item?: MenuItem | null
}>()

const emit = defineEmits<{
  save: [item: MenuItem]
  cancel: []
}>()

type ActionType = MenuAction['type']

type FormState = {
  id: string
  label: string
  hint: string
  icon: string
  accent: string
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
  actionType: 'program',
  value: '',
  systemTarget: 'explorer',
})

const isEditing = computed(() => Boolean(props.item))
const requiresValue = computed(() => form.actionType !== 'system')
const canSave = computed(() =>
  form.label.trim()
  && form.hint.trim()
  && form.icon.trim()
  && (!requiresValue.value || form.value.trim()),
)

watch(
  () => props.item,
  (item) => {
    form.id = item?.id ?? ''
    form.label = item?.label ?? ''
    form.hint = item?.hint ?? ''
    form.icon = item?.icon ?? ''
    form.accent = item?.accent ?? '#8b7cff'
    form.actionType = item?.action.type ?? 'program'
    form.systemTarget = item?.action.type === 'system' ? item.action.target : 'explorer'
    form.value = getActionValue(item?.action)
  },
  { immediate: true },
)

function getActionValue(action?: MenuAction) {
  if (!action || action.type === 'system') return ''
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

function buildAction(): MenuAction {
  if (form.actionType === 'program') return { type: 'program', path: form.value.trim() }
  if (form.actionType === 'directory') return { type: 'directory', path: form.value.trim() }
  if (form.actionType === 'url') return { type: 'url', url: form.value.trim() }
  return { type: 'system', target: form.systemTarget }
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
          <span>{{ isEditing ? 'EDITAR AÇÃO' : 'NOVA AÇÃO' }}</span>
          <h2>{{ isEditing ? 'Editar item do menu' : 'Adicionar ao menu radial' }}</h2>
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

        <label class="item-modal__wide">
          <span>Tipo de ação</span>
          <select v-model="form.actionType">
            <option value="program">Abrir programa</option>
            <option value="directory">Abrir diretório</option>
            <option value="url">Abrir URL</option>
            <option value="system">Ação padrão do sistema</option>
          </select>
        </label>

        <label v-if="form.actionType === 'system'" class="item-modal__wide">
          <span>Ação padrão</span>
          <select v-model="form.systemTarget">
            <option value="explorer">Explorador de arquivos</option>
            <option value="default_browser">Navegador padrão</option>
            <option value="terminal">Terminal</option>
            <option value="calculator">Calculadora</option>
            <option value="notepad">Bloco de notas</option>
          </select>
        </label>

        <label v-else class="item-modal__wide">
          <span>
            {{ form.actionType === 'url' ? 'URL' : 'Caminho' }}
          </span>
          <span class="path-field">
            <input
              v-model="form.value"
              required
              :type="form.actionType === 'url' ? 'url' : 'text'"
              :placeholder="form.actionType === 'url' ? 'https://exemplo.com' : 'C:\\Caminho'"
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
          {{ isEditing ? 'Salvar alterações' : 'Adicionar ação' }}
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
