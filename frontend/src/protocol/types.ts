export const MAX_WAV_BYTES = 512 * 1024
export const MAX_CONTROL_BYTES = 4096
export const MAX_AUDIO_SECONDS = 15
export const UPLOAD_TIMEOUT_MS = 15_000
export const SAMPLE_RATE = 16_000
export const NAME_MAX_CHARS = 40

export type UserId = string
export type MessageId = string
export type RequestId = string

export interface UserInfo {
  id: UserId
  name: string
  busy: boolean
}

export type ClientMessage =
  | { type: 'set_name'; name: string }
  | {
      type: 'begin_upload'
      recipient_id: UserId
      request_id: RequestId
      byte_length: number
    }

export type ErrorCode =
  | 'invalid_message'
  | 'invalid_name'
  | 'upload_pending'
  | 'invalid_recipient'
  | 'unknown_recipient'
  | 'unavailable'
  | 'invalid_size'
  | 'unexpected_audio'
  | 'size_mismatch'
  | 'invalid_wav'
  | 'recipient_unavailable'
  | 'upload_timeout'

export type ServerMessage =
  | { type: 'welcome'; user_id: UserId }
  | { type: 'users'; users: UserInfo[] }
  | {
      type: 'audio_message'
      message_id: MessageId
      sender_id: UserId
      recipient_id: UserId
      byte_length: number
    }
  | { type: 'upload_ready'; request_id: RequestId; message_id: MessageId }
  | { type: 'transcript_ready'; message_id: MessageId; transcript_body: string }
  | { type: 'transcript_failed'; message_id: MessageId; reason: string }
  | { type: 'message_accepted'; message_id: MessageId }
  | { type: 'error'; code: ErrorCode; message: string }

const SERVER_TYPES = new Set([
  'welcome',
  'users',
  'audio_message',
  'upload_ready',
  'transcript_ready',
  'transcript_failed',
  'message_accepted',
  'error',
])


export function parseServerMessage(raw: string): ServerMessage | null {
  try {
    const value: unknown = JSON.parse(raw)
    if (typeof value !== 'object' || value === null) return null
    const type = (value as { type?: unknown }).type
    if (typeof type !== 'string' || !SERVER_TYPES.has(type)) return null
    return value as ServerMessage
  } catch {
    return null
  }
}
