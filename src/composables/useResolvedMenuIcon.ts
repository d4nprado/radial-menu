import { invoke } from '@tauri-apps/api/core'
import type { MenuItem, StreamAction, SystemActionTarget } from '../types/menu'

export type FixedIconKind =
  | 'folder'
  | 'globe'
  | 'terminal'
  | 'calculator'
  | 'notepad'
  | 'scene'
  | 'record'
  | 'broadcast'
  | 'mic'
  | 'mic-muted'
  | 'speaker'
  | 'speaker-muted'
  | 'audio'
  | 'audio-muted'
  | 'source'
  | 'source-hidden'
  | 'camera'
  | 'camera-off'
  | 'group'

export type ProgramIconPayload = {
  width: number
  height: number
  rgba: number[]
}

const programIconCache = new Map<string, Promise<ProgramIconPayload | null>>()

const systemIcons: Record<SystemActionTarget, FixedIconKind> = {
  explorer: 'folder',
  default_browser: 'globe',
  terminal: 'terminal',
  calculator: 'calculator',
  notepad: 'notepad',
}

export function fixedIconForItem(item: MenuItem): FixedIconKind | null {
  if (item.action.type === 'directory') return 'folder'
  if (item.action.type === 'url') return 'globe'
  if (item.action.type === 'system') return systemIcons[item.action.target]
  if (item.action.type === 'stream') return streamBaseIcon(item.action)
  if (item.action.type === 'group') return 'group'
  return null
}

export function streamBaseIcon(action: StreamAction): FixedIconKind {
  if (action.operation === 'set_scene') return 'scene'
  if (
    action.operation === 'start_recording'
    || action.operation === 'stop_recording'
    || action.operation === 'toggle_recording'
  ) return 'record'
  if (
    action.operation === 'start_streaming'
    || action.operation === 'stop_streaming'
    || action.operation === 'toggle_streaming'
  ) return 'broadcast'
  if (action.operation === 'set_input_mute' || action.operation === 'toggle_input_mute') {
    return audioInputBaseIcon(action.inputName ?? '', action.muted)
  }
  if (action.operation === 'set_source_visibility' || action.operation === 'toggle_source_visibility') {
    return sourceBaseIcon(action.sourceName ?? '', action.visible)
  }
  return 'broadcast'
}

export function audioInputKind(inputName: string): 'mic' | 'speaker' | 'audio' {
  const normalized = normalizeIconTarget(inputName)

  if (/\b(mic|microphone|microfone|mic\/aux|aux)\b/.test(normalized)) return 'mic'
  if (
    /\b(desktop|speaker|speakers|alto[- ]?falante|volume|output|saida|audio do desktop)\b/
      .test(normalized)
  ) return 'speaker'
  return 'audio'
}

export function sourceLooksLikeCamera(sourceName: string) {
  return /\b(cam|camera|webcam|video|capture)\b/.test(normalizeIconTarget(sourceName))
}

function audioInputBaseIcon(inputName: string, muted?: boolean): FixedIconKind {
  const kind = audioInputKind(inputName)
  if (kind === 'mic') return muted ? 'mic-muted' : 'mic'
  if (kind === 'speaker') return muted ? 'speaker-muted' : 'speaker'
  return muted ? 'audio-muted' : 'audio'
}

function sourceBaseIcon(sourceName: string, visible?: boolean): FixedIconKind {
  const hidden = visible === false
  if (sourceLooksLikeCamera(sourceName)) return hidden ? 'camera-off' : 'camera'
  return hidden ? 'source-hidden' : 'source'
}

function normalizeIconTarget(value: string) {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
}

export function fallbackTextForItem(item: MenuItem) {
  const configured = item.icon.trim()
  if (configured) return configured

  return item.label
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0] ?? '')
    .join('')
    .toUpperCase()
    .slice(0, 3) || '?'
}

export function resolveProgramIcon(path: string) {
  const cacheKey = path.trim().toLowerCase()
  if (!cacheKey) return Promise.resolve(null)

  const cached = programIconCache.get(cacheKey)
  if (cached) return cached

  const pending = invoke<ProgramIconPayload | null>('extract_program_icon', { path })
    .then((payload) => isValidPayload(payload) ? payload : null)
    .catch(() => null)
  programIconCache.set(cacheKey, pending)
  return pending
}

function isValidPayload(
  payload: ProgramIconPayload | null,
): payload is ProgramIconPayload {
  return Boolean(
    payload
    && payload.width > 0
    && payload.height > 0
    && payload.rgba.length === payload.width * payload.height * 4,
  )
}
