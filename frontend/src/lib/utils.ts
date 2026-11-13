import type { ConnectionStatus } from '@/net/socket'
import type { UserId, UserInfo } from '@/protocol/types'
import type { ChatsState } from '@/store/chats'
import { failureText } from '@/store/upload'
import type { UploadState } from '@/store/upload'

export { cn } from 'cn'

export type PeerUser = UserInfo & { online: boolean }

export function formatUsers(
  users: UserInfo[],
  status: ConnectionStatus,
  selfId: UserId | null,
  byPeer: ChatsState['byPeer'],
): PeerUser[] {
  const peers: PeerUser[] =
    status === 'open'
      ? users.filter((user) => user.id !== selfId).map((user) => ({ ...user, online: true }))
      : []
  const knownIds = new Set(peers.map((user) => user.id))

  for (const [id, messages] of Object.entries(byPeer)) {
    const lastMessage = messages.at(-1)
    if (id !== selfId && !knownIds.has(id) && lastMessage) {
      peers.push({ id, name: lastMessage.peerName, busy: false, online: false })
    }
  }

  return peers
}

export function getRecordButtonLabels(
  peerOnline: boolean,
  connected: boolean,
  upload: UploadState,
): { label: string; hint: string } {
  if (!peerOnline) {
    return { label: 'They left', hint: 'Messages stay until you reload' }
  }
  if (!connected) {
    return { label: 'Reconnecting…', hint: '' }
  }
  if (upload.phase === 'recording') {
    return { label: 'Recording', hint: 'Release to send' }
  }
  if (upload.phase === 'requesting' || upload.phase === 'uploading') {
    return { label: 'Sending…', hint: '' }
  }
  if (upload.phase === 'failed') {
    return { label: 'Hold to talk', hint: failureText(upload.code) }
  }
  return { label: 'Hold to talk', hint: 'Hold Space or this button' }
}
