import { create } from 'zustand'
import type { MessageId, UserId } from '@/protocol/types'

export type Transcript =
  { status: 'pending' } | { status: 'ready'; text: string } | { status: 'failed' }

export interface ChatMessage {
  id: MessageId
  peerId: UserId
  peerName: string
  mine: boolean
  wav: ArrayBuffer
  at: number
  transcript: Transcript
}

export interface ChatsState {
  byPeer: Record<UserId, ChatMessage[]>
}

export const useChats = create<ChatsState>()(() => ({ byPeer: {} }))

export function addMessage(message: ChatMessage): void {
  useChats.setState(({ byPeer }) => ({
    byPeer: {
      ...byPeer,
      [message.peerId]: [...(byPeer[message.peerId] ?? []), message],
    },
  }))
}

export function setTranscript(id: MessageId, transcript: Transcript): void {
  useChats.setState(({ byPeer }) => {
    const next: ChatsState['byPeer'] = {}
    for (const [peerId, messages] of Object.entries(byPeer)) {
      next[peerId] = messages.map((message) =>
        message.id === id ? { ...message, transcript } : message,
      )
    }
    return { byPeer: next }
  })
}

export function clearChats(): void {
  useChats.setState({ byPeer: {} })
}

export function transcriptText(transcript: Transcript): string {
  if (transcript.status === 'ready') return transcript.text
  if (transcript.status === 'pending') return 'Transcribing…'
  return 'Transcript unavailable'
}
