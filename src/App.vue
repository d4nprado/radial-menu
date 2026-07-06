<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import RadialMenu from './components/RadialMenu.vue'
import { useMenuActions } from './composables/useMenuActions'
import { useSystemStats } from './composables/useSystemStats'
import type {
  MenuConfig,
  MenuItem,
  PreferencesLoadResponse,
} from './types/menu'

const menuConfig = ref<MenuConfig>({ shortcut: 'Ctrl+Space', items: [] })
const phase = ref<'entering' | 'visible' | 'leaving'>('entering')
const { execute, isExecuting, error } = useMenuActions()
const { stats, start: startStats, stop: stopStats } = useSystemStats()
const unlisteners: UnlistenFn[] = []
let hideTimer: number | undefined

function showAnimation() {
  window.clearTimeout(hideTimer)
  startStats()
  phase.value = 'entering'
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      phase.value = 'visible'
    })
  })
}

function dismiss() {
  if (phase.value === 'leaving') return

  stopStats()
  phase.value = 'leaving'
  hideTimer = window.setTimeout(() => {
    void invoke('hide_menu')
  }, 130)
}

async function selectItem(item: MenuItem) {
  try {
    await execute(item.action)
    dismiss()
  } catch {
    window.setTimeout(() => {
      error.value = null
    }, 2400)
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') dismiss()
}

function onWindowBlur() {
  dismiss()
}

onMounted(async () => {
  try {
    const response = await invoke<{ config: MenuConfig; warning: string | null }>(
      'load_launcher_config',
    )
    menuConfig.value = response.config
    if (response.warning) error.value = response.warning

    const preferences = await invoke<PreferencesLoadResponse>('get_app_preferences')
    menuConfig.value.shortcut = preferences.preferences.openMenuShortcut.value
  } catch (cause) {
    error.value = typeof cause === 'string'
      ? cause
      : 'Não foi possível carregar a configuração salva.'
  }

  unlisteners.push(await listen<MenuConfig>('launcher-config-updated', (event) => {
    menuConfig.value = {
      ...event.payload,
      shortcut: menuConfig.value.shortcut,
    }
  }))
  unlisteners.push(await listen<string>('launcher-shortcut-updated', (event) => {
    menuConfig.value.shortcut = event.payload
  }))
  unlisteners.push(await listen('menu:show', showAnimation))
  unlisteners.push(await listen('menu:hide', () => {
    stopStats()
    phase.value = 'leaving'
  }))
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('blur', onWindowBlur)
})

onBeforeUnmount(() => {
  window.clearTimeout(hideTimer)
  unlisteners.forEach((unlisten) => unlisten())
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('blur', onWindowBlur)
})
</script>

<template>
  <main>
    <RadialMenu
      :items="menuConfig.items"
      :shortcut="menuConfig.shortcut"
      :phase="phase"
      :stats="stats"
      :disabled="isExecuting"
      @select="selectItem"
      @dismiss="dismiss"
    />

    <Transition name="toast">
      <p v-if="error" class="error-toast">{{ error }}</p>
    </Transition>
  </main>
</template>
