export type ProgramAction = {
  type: 'program'
  path: string
}

export type DirectoryAction = {
  type: 'directory'
  path: string
}

// Add future action contracts (for example "obs") to this union.
export type MenuAction = ProgramAction | DirectoryAction

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

