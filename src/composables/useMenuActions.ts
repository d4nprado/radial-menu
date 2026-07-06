import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { MenuAction } from '../types/menu'

type ActionExecutor<T extends MenuAction = MenuAction> = (
  action: T,
) => Promise<void>

const executors: {
  [K in MenuAction['type']]: ActionExecutor<Extract<MenuAction, { type: K }>>
} = {
  program: (action) =>
    invoke('open_program', { path: action.path }),
  directory: (action) =>
    invoke('open_directory', { path: action.path }),
  url: (action) =>
    invoke('open_url', { url: action.url }),
  system: (action) =>
    invoke('execute_system_action', { target: action.target }),
  group: () =>
    Promise.reject('Grupos devem ser abertos pelo menu, não executados como ações.'),
}

export function useMenuActions() {
  const isExecuting = ref(false)
  const error = ref<string | null>(null)

  async function execute(action: MenuAction) {
    if (isExecuting.value) return

    isExecuting.value = true
    error.value = null

    try {
      const executor = executors[action.type] as ActionExecutor
      await executor(action)
    } catch (cause) {
      error.value =
        typeof cause === 'string' ? cause : 'Não foi possível executar a ação.'
      throw cause
    } finally {
      isExecuting.value = false
    }
  }

  return { execute, isExecuting, error }
}
