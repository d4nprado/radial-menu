import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FixedIconKind } from './useResolvedMenuIcon'
import type { MenuItem, ObsStreamStatus, StreamAction } from '../types/menu'

export type StreamToggleState = {
  active: boolean
  hint: string
  icon?: FixedIconKind
  showInactiveLed?: boolean
}

export function streamToggleState(
  action: StreamAction,
  status: ObsStreamStatus | null,
): StreamToggleState | null {
  if (action.provider !== 'obs') return null
  if (!status) return null

  if (action.operation === 'toggle_recording') {
    const active = status.recording.active
    return {
      active,
      hint: active ? 'Gravando' : 'Gravação parada',
      showInactiveLed: true,
    }
  }

  if (action.operation === 'toggle_streaming') {
    const active = status.streaming.active
    return {
      active,
      hint: active ? 'Ao vivo' : 'Transmissão offline',
      showInactiveLed: true,
    }
  }

  if (action.operation === 'toggle_input_mute') {
    const inputName = action.inputName?.trim()
    if (!inputName || !(inputName in status.inputMutes)) return null

    const muted = status.inputMutes[inputName] ?? false
    const audioKind = audioInputKind(inputName)
    return {
      active: muted,
      hint: audioHint(inputName, audioKind, muted),
      icon: audioIcon(audioKind, muted),
      showInactiveLed: false,
    }
  }

  return null
}

export function itemStreamToggleState(
  item: MenuItem,
  status: ObsStreamStatus | null,
): StreamToggleState | null {
  if (item.action.type !== 'stream') return null
  return streamToggleState(item.action, status)
}

export function useObsStreamStatus() {
  const status = ref<ObsStreamStatus | null>(null)
  const isLoading = ref(false)

  async function refresh(inputNames: string[] = []) {
    isLoading.value = true
    try {
      const nextStatus = await invoke<ObsStreamStatus>('get_obs_stream_status')
      const uniqueInputNames = Array.from(new Set(
        inputNames
          .map((inputName) => inputName.trim())
          .filter(Boolean),
      ))
      if (uniqueInputNames.length) {
        nextStatus.inputMutes = await invoke<Record<string, boolean>>(
          'get_obs_input_mute_statuses',
          { inputNames: uniqueInputNames },
        )
      }
      status.value = nextStatus
    } catch (cause) {
      console.warn('Não foi possível consultar o status do OBS.', cause)
      status.value = null
    } finally {
      isLoading.value = false
    }
  }

  function clear() {
    status.value = null
  }

  return { status, isLoading, refresh, clear }
}

function audioInputKind(inputName: string): 'mic' | 'speaker' | 'audio' {
  const normalized = inputName
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()

  if (/\b(mic|microphone|microfone|mic\/aux|aux)\b/.test(normalized)) return 'mic'
  if (
    /\b(desktop|speaker|speakers|alto[- ]?falante|volume|output|saida|audio do desktop)\b/
      .test(normalized)
  ) return 'speaker'
  return 'audio'
}

function audioIcon(kind: 'mic' | 'speaker' | 'audio', muted: boolean): FixedIconKind {
  if (kind === 'mic') return muted ? 'mic-muted' : 'mic'
  if (kind === 'speaker') return muted ? 'speaker-muted' : 'speaker'
  return muted ? 'audio-muted' : 'audio'
}

function audioHint(inputName: string, kind: 'mic' | 'speaker' | 'audio', muted: boolean) {
  const base = kind === 'mic'
    ? 'MIC'
    : kind === 'speaker'
      ? 'DESKTOP'
      : inputName.trim().toUpperCase()
  return muted ? `${base} MUTADO` : base
}
