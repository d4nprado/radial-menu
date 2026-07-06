import { invoke } from '@tauri-apps/api/core'
import type { MenuItem, SystemActionTarget } from '../types/menu'

export type FixedIconKind =
  | 'folder'
  | 'globe'
  | 'terminal'
  | 'calculator'
  | 'notepad'
  | 'broadcast'
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
  if (item.action.type === 'stream') return 'broadcast'
  if (item.action.type === 'group') return 'group'
  return null
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
