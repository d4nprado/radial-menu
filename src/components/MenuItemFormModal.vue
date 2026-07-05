<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import type { MenuAction, MenuItem, SystemActionTarget } from '../types/menu'

const props = defineProps<{ item?: MenuItem | null }>()
const emit = defineEmits<{ save: [item: MenuItem]; cancel: [] }>()
type ActionType = MenuAction['type']
const form = reactive({
  id: '', label: '', hint: '', icon: '', accent: '#8b7cff',
  actionType: 'program' as ActionType, value: '', systemTarget: 'explorer' as SystemActionTarget,
})
const valid = computed(() => Boolean(form.label.trim() && form.hint.trim() && form.icon.trim() && (form.actionType === 'system' || form.value.trim())))

watch(() => props.item, (item) => {
  Object.assign(form, {
    id: item?.id ?? '', label: item?.label ?? '', hint: item?.hint ?? '', icon: item?.icon ?? '',
    accent: item?.accent ?? '#8b7cff', actionType: item?.action.type ?? 'program',
    value: !item || item.action.type === 'system' ? '' : item.action.type === 'url' ? item.action.url : item.action.path,
    systemTarget: item?.action.type === 'system' ? item.action.target : 'explorer',
  })
}, { immediate: true })

async function choose(directory: boolean) {
  const selected = await open(directory
    ? { multiple: false, directory: true }
    : { multiple: false, directory: false, filters: [{ name: 'Programas', extensions: ['exe', 'com', 'bat', 'cmd'] }] })
  if (typeof selected === 'string') form.value = selected
}

function action(): MenuAction {
  if (form.actionType === 'program') return { type: 'program', path: form.value.trim() }
  if (form.actionType === 'directory') return { type: 'directory', path: form.value.trim() }
  if (form.actionType === 'url') return { type: 'url', url: form.value.trim() }
  return { type: 'system', target: form.systemTarget }
}

function submit() {
  if (valid.value) emit('save', { id: form.id, label: form.label.trim(), hint: form.hint.trim(), icon: form.icon.trim(), accent: form.accent, action: action() })
}
</script>

<template>
  <div class="backdrop" @mousedown.self="emit('cancel')">
    <form @submit.prevent="submit">
      <header><div><small>{{ item ? 'EDITAR AÇÃO' : 'NOVA AÇÃO' }}</small><h2>{{ item ? 'Editar item do menu' : 'Adicionar ao menu radial' }}</h2></div><button type="button" @click="emit('cancel')">×</button></header>
      <div class="grid">
        <label><span>Label</span><input v-model="form.label" required placeholder="Visual Studio Code"></label>
        <label><span>Hint</span><input v-model="form.hint" required placeholder="Desenvolvimento"></label>
        <label><span>Ícone / texto curto</span><input v-model="form.icon" required maxlength="3" placeholder="VS"></label>
        <label><span>Cor / accent</span><span class="row color"><input v-model="form.accent" type="color"><input v-model="form.accent" pattern="#[0-9a-fA-F]{6}"></span></label>
        <label class="wide"><span>Tipo de ação</span><select v-model="form.actionType"><option value="program">Abrir programa</option><option value="directory">Abrir diretório</option><option value="url">Abrir URL</option><option value="system">Ação padrão do sistema</option></select></label>
        <label v-if="form.actionType === 'system'" class="wide"><span>Ação padrão</span><select v-model="form.systemTarget"><option value="explorer">Explorador de arquivos</option><option value="default_browser">Navegador padrão</option><option value="terminal">Terminal</option><option value="calculator">Calculadora</option><option value="notepad">Bloco de notas</option></select></label>
        <label v-else class="wide"><span>{{ form.actionType === 'url' ? 'URL' : 'Caminho' }}</span><span class="row"><input v-model="form.value" required :type="form.actionType === 'url' ? 'url' : 'text'" :placeholder="form.actionType === 'url' ? 'https://exemplo.com' : 'C:\\Caminho'"><button v-if="form.actionType === 'program'" type="button" @click="choose(false)">Selecionar programa</button><button v-if="form.actionType === 'directory'" type="button" @click="choose(true)">Selecionar pasta</button></span></label>
      </div>
      <footer><button type="button" @click="emit('cancel')">Cancelar</button><button class="primary" :disabled="!valid">{{ item ? 'Salvar alterações' : 'Adicionar ação' }}</button></footer>
    </form>
  </div>
</template>

<style scoped>
.backdrop{position:fixed;z-index:30;inset:0;display:grid;padding:30px;place-items:center;background:#04060cb8;backdrop-filter:blur(8px)}form{width:min(100%,650px);max-height:100%;padding:26px;overflow-y:auto;border:1px solid #ffffff1c;border-radius:20px;color:#f2f3fa;background:#151827;box-shadow:0 30px 90px #0008}header,footer,.row{display:flex}header{align-items:flex-start;justify-content:space-between}header small{color:#8d82ff;font-size:10px;font-weight:700;letter-spacing:.14em}h2{margin:5px 0 0;font-size:21px}header>button{width:32px;height:32px;border:0;border-radius:9px;color:#9297aa;background:#ffffff0d;font-size:20px;cursor:pointer}.grid{display:grid;margin-top:24px;grid-template-columns:1fr 1fr;gap:16px}label{display:flex;min-width:0;flex-direction:column;gap:7px}label>span:first-child{color:#a5a9ba;font-size:10px;font-weight:600}.wide{grid-column:1/-1}input,select{width:100%;min-width:0;height:40px;padding:0 12px;border:1px solid #ffffff1a;border-radius:9px;outline:0;color:#f0f1f8;background:#0e111d}input:focus,select:focus{border-color:#8b7cffb3;box-shadow:0 0 0 3px #8b7cff1c}.row{gap:8px}.row button{padding:0 13px;flex:none;border:1px solid #8b7cff40;border-radius:9px;color:#bcb6ff;background:#8b7cff17;cursor:pointer}.color input[type=color]{width:46px;padding:5px;flex:none}footer{margin-top:25px;padding-top:18px;justify-content:flex-end;border-top:1px solid #ffffff12;gap:9px}footer button{padding:10px 16px;border:1px solid #ffffff1a;border-radius:9px;color:#bfc2d0;background:transparent;cursor:pointer}.primary{border-color:#7567e8;color:#fff;background:linear-gradient(135deg,#796bea,#5f51d0)}button:disabled{cursor:not-allowed;opacity:.4}
</style>
