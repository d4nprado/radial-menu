<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import RadialMenu from './components/RadialMenu.vue'
import { useMenuActions } from './composables/useMenuActions'
import { useSystemStats } from './composables/useSystemStats'
import menuConfigJson from './config/menu.json'
import type { MenuConfig, MenuItem } from './types/menu'

const menuConfig = menuConfigJson as MenuConfig
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
