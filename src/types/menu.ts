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

// Add future action contracts (for example "obs") to this union.
export type MenuAction =
  | ProgramAction
  | DirectoryAction
  | UrlAction
  | SystemAction

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
  items: MenuItem[]
}
