import { create } from 'zustand'
import { recorder, socket } from '@/net/client'
import type { ErrorCode, MessageId, RequestId, UserId } from '@/protocol/types'
import { addMessage } from './chats'
import { userName } from './session'

export type FailureCode = ErrorCode | 'disconnected' | 'mic_unavailable'

/**
 * One outgoing message at a time (the backend allows one pending upload).
 *
 *   idle ─press→ recording ─release→ requesting ─upload_ready→ uploading
 *        ─message_accepted→ idle
 *
 * Any error on the way lands in `failed`, which behaves like idle.
 */
export type UploadState =
  | { phase: 'idle' }
  | { phase: 'recording'; peerId: UserId }
  | { phase: 'requesting'; peerId: UserId; requestId: RequestId; wav: ArrayBuffer }
  | { phase: 'uploading'; peerId: UserId; messageId: MessageId; wav: ArrayBuffer }
  | { phase: 'failed'; peerId: UserId; code: FailureCode }

export const useUpload = create<UploadState>()(() => ({ phase: 'idle' }))

function set(next: UploadState): void {
  useUpload.setState(next, true)
}

export async function startTalk(peerId: UserId): Promise<void> {
  const { phase } = useUpload.getState()
  if (phase !== 'idle' && phase !== 'failed') return

  set({ phase: 'recording', peerId })
  recorder.onLimit = () => void stopTalk()
  try {
    await recorder.start()
  } catch {
    if (useUpload.getState().phase === 'recording') {
      set({ phase: 'failed', peerId, code: 'mic_unavailable' })
    }
  }
}

export async function stopTalk(): Promise<void> {
  const before = useUpload.getState()
  if (before.phase !== 'recording') return
  const { peerId } = before

  const wav = await recorder.stop()
  if (useUpload.getState().phase !== 'recording') return
  if (!wav) return set({ phase: 'idle' })

  const requestId = crypto.randomUUID()
  const sent = socket.send({
    type: 'begin_upload',
    recipient_id: peerId,
    request_id: requestId,
    byte_length: wav.byteLength,
  })
  set(
    sent
      ? { phase: 'requesting', peerId, requestId, wav }
      : { phase: 'failed', peerId, code: 'disconnected' },
  )
}

export function handleUploadReady(requestId: RequestId, messageId: MessageId): void {
  const state = useUpload.getState()
  if (state.phase !== 'requesting' || state.requestId !== requestId) return
  const { peerId, wav } = state

  set(
    socket.sendAudio(wav)
      ? { phase: 'uploading', peerId, messageId, wav }
      : { phase: 'failed', peerId, code: 'disconnected' },
  )
}

export function handleAccepted(messageId: MessageId): void {
  const state = useUpload.getState()
  if (state.phase !== 'uploading' || state.messageId !== messageId) return

  addMessage({
    id: messageId,
    peerId: state.peerId,
    peerName: userName(state.peerId),
    mine: true,
    wav: state.wav,
    at: Date.now(),
    transcript: { status: 'pending' },
  })
  set({ phase: 'idle' })
}

export function handleError(code: ErrorCode): void {
  const state = useUpload.getState()
  if (state.phase !== 'requesting' && state.phase !== 'uploading') return
  set({ phase: 'failed', peerId: state.peerId, code })
}

export function handleDisconnect(): void {
  const state = useUpload.getState()
  if (state.phase === 'recording') {
    void recorder.stop()
    set({ phase: 'idle' })
  } else if (state.phase === 'requesting' || state.phase === 'uploading') {
    set({ phase: 'failed', peerId: state.peerId, code: 'disconnected' })
  }
}

export function failureText(code: FailureCode): string {
  switch (code) {
    case 'mic_unavailable':
      return 'Microphone unavailable. Check browser permissions.'
    case 'disconnected':
      return 'Connection lost. Message not sent.'
    case 'unknown_recipient':
    case 'recipient_unavailable':
    case 'unavailable':
      return 'They left before the message arrived.'
    case 'upload_timeout':
      return 'Sending took too long. Try again.'
    default:
      return 'Message not sent. Try again.'
  }
}
