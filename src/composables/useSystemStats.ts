import { onBeforeUnmount, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SystemStats } from '../types/systemStats'

const REFRESH_INTERVAL_MS = 1_000

export function useSystemStats() {
  const stats = ref<SystemStats | null>(null)
  let intervalId: number | undefined
  let session = 0

  async function refresh(currentSession: number) {
    try {
      const nextStats = await invoke<SystemStats>('get_system_stats')
      if (session === currentSession) stats.value = nextStats
    } catch {
      if (session === currentSession) stats.value = null
    }
  }

  function start() {
    stop()
    stats.value = null
    const currentSession = session
    void refresh(currentSession)
    intervalId = window.setInterval(() => {
      void refresh(currentSession)
    }, REFRESH_INTERVAL_MS)
  }

  function stop() {
    session += 1
    window.clearInterval(intervalId)
    intervalId = undefined
  }

  onBeforeUnmount(stop)

  return { stats, start, stop }
}
