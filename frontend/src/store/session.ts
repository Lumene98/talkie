import { create } from 'zustand'
import { socket } from '@/net/client'
import type { ConnectionStatus } from '@/net/socket'
import { NAME_MAX_CHARS } from '@/protocol/types'
import type { UserId, UserInfo } from '@/protocol/types'

export type View = 'people' | 'chat' | 'chats'

export interface SessionState {
  status: ConnectionStatus
  selfId: UserId | null
  name: string
  users: UserInfo[]
  view: View
  activePeerId: UserId | null
  nameDialogOpen: boolean
}

const NAME_KEY = 'talkie-name'

function savedName(): string {
  try {
    return localStorage.getItem(NAME_KEY) ?? ''
  } catch {
    return ''
  }
}

export const useSession = create<SessionState>()(() => ({
  status: 'closed',
  selfId: null,
  name: savedName(),
  users: [],
  view: 'people',
  activePeerId: null,
  nameDialogOpen: savedName() === '',
}))

export function setName(input: string): string | null {
  const name = input.trim()
  if (name === '') return 'Enter a name.'
  if ([...name].length > NAME_MAX_CHARS) return `Use ${NAME_MAX_CHARS} characters or fewer.`

  localStorage.setItem(NAME_KEY, name)

  useSession.setState({ name, nameDialogOpen: false })
  // If the socket is down this is skipped; the name is sent again on `welcome`.
  socket.send({ type: 'set_name', name })
  return null
}

export function setNameDialogOpen(nameDialogOpen: boolean): void {
  useSession.setState({ nameDialogOpen })
}

export function setView(view: View): void {
  useSession.setState({ view })
}

export function openChat(peerId: UserId): void {
  useSession.setState({ activePeerId: peerId, view: 'chat' })
}

export function userName(id: UserId): string {
  return useSession.getState().users.find((user) => user.id === id)?.name ?? 'Unknown'
}
