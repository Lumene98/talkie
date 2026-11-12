import { parseServerMessage } from '@/protocol/types'
import type { ClientMessage, ServerMessage } from '@/protocol/types'

export type ConnectionStatus = 'connecting' | 'open' | 'closed'
export type AudioMeta = Extract<ServerMessage, { type: 'audio_message' }>

export interface SocketHandlers {
  onStatus(status: ConnectionStatus): void
  onMessage(message: ServerMessage): void
  onAudio(meta: AudioMeta, wav: ArrayBuffer): void
}

export class TalkieSocket {
  handlers: SocketHandlers | null = null
  private ws: WebSocket | null = null
  private pendingAudio: AudioMeta | null = null
  private retryMs = 1000

  connect(): void {
    if (this.ws) return
    this.handlers?.onStatus('connecting')

    const scheme = location.protocol === 'https:' ? 'wss' : 'ws'
    const ws = new WebSocket(`${scheme}://${location.host}/ws`)
    ws.binaryType = 'arraybuffer'
    this.ws = ws

    ws.onopen = () => {
      this.retryMs = 1000
      this.handlers?.onStatus('open')
    }

    ws.onmessage = (event: MessageEvent<string | ArrayBuffer>) => {
      if (typeof event.data === 'string') {
        const message = parseServerMessage(event.data)
        if (!message) return
        // The WAV is the next frame; hold the metadata until it arrives.
        if (message.type === 'audio_message') this.pendingAudio = message
        else this.handlers?.onMessage(message)
        return
      }
      const meta = this.pendingAudio
      this.pendingAudio = null
      if (meta) this.handlers?.onAudio(meta, event.data)
    }

    ws.onclose = () => {
      this.ws = null
      this.pendingAudio = null
      this.handlers?.onStatus('closed')
      setTimeout(() => this.connect(), this.retryMs)
      this.retryMs = Math.min(this.retryMs * 2, 10_000)
    }
  }

  send(message: ClientMessage): boolean {
    return this.sendRaw(JSON.stringify(message))
  }

  sendAudio(wav: ArrayBuffer): boolean {
    return this.sendRaw(wav)
  }

  private sendRaw(data: string | ArrayBuffer): boolean {
    if (this.ws?.readyState !== WebSocket.OPEN) return false
    this.ws.send(data)
    return true
  }
}
