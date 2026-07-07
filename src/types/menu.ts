export type ProgramAction = {
  type: 'program'
  path: string
}

export type DirectoryAction = {
  type: 'directory'
  path: string
}

export type UrlAction = {
  type: 'url'
  url: string
}

export type SystemActionTarget =
  | 'explorer'
  | 'default_browser'
  | 'terminal'
  | 'calculator'
  | 'notepad'

export type SystemAction = {
  type: 'system'
  target: SystemActionTarget
}

export type StreamOperation =
  | 'set_scene'
  | 'start_recording'
  | 'stop_recording'
  | 'toggle_recording'
  | 'start_streaming'
  | 'stop_streaming'
  | 'toggle_streaming'
  | 'set_input_mute'
  | 'toggle_input_mute'
  | 'set_source_visibility'
  | 'toggle_source_visibility'

export type StreamAction = {
  type: 'stream'
  provider: 'obs'
  operation: StreamOperation
  sceneName?: string
  inputName?: string
  sourceName?: string
  muted?: boolean
  visible?: boolean
}

export type GroupAction = {
  type: 'group'
  items: MenuItem[]
}

export type MenuAction =
  | ProgramAction
  | DirectoryAction
  | UrlAction
  | SystemAction
  | StreamAction
  | GroupAction

export type MenuItem = {
  id: string
  label: string
  hint: string
  icon: string
  accent: string
  action: MenuAction
}

export type MenuConfig = {
  shortcut: string
  radialMenuSize: number
  items: MenuItem[]
}

export const MAX_MENU_ITEMS_PER_LEVEL = 10

export type CenterAction = 'close' | 'back'

export type ConfigLoadResponse = {
  config: MenuConfig
  warning: string | null
}

export type AppPreferences = {
  startWithWindows: boolean
  openMenuShortcut: {
    type: 'keyboard' | 'mouse'
    value: string
  }
}

export type PreferencesLoadResponse = {
  preferences: AppPreferences
  warning: string | null
}

export type StreamPreferences = {
  obs: {
    host: string
    port: number
    password: string
  }
}

export type StreamPreferencesLoadResponse = {
  preferences: StreamPreferences
  warning: string | null
}

export type ObsConnectionStatus = {
  ok: boolean
  message: string
}

export type ObsStreamStatus = {
  recording: {
    active: boolean
  }
  streaming: {
    active: boolean
  }
  inputMutes: Record<string, boolean>
}
