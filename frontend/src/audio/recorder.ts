import RecordRTC from 'recordrtc'
import { MAX_AUDIO_SECONDS, SAMPLE_RATE } from '@/protocol/types'

// Stop half a second early: the backend rejects anything over 15 s, and the
// timer and the recorder's internal buffering are not exact.
const LIMIT_MS = MAX_AUDIO_SECONDS * 1000 - 500
const MIN_MS = 300 // shorter holds are accidental taps

interface Take {
  stream: MediaStream
  rtc: RecordRTC
  startedAt: number
  limitTimer: number
}

export class Recorder {
  onLimit: (() => void) | null = null
  private take: Promise<Take> | null = null

  async start(): Promise<void> {
    this.take = this.open()
    await this.take
  }

  async stop(): Promise<ArrayBuffer | null> {
    const pending = this.take
    this.take = null
    if (!pending) return null

    let take: Take
    try {
      take = await pending
    } catch {
      return null
    }
    clearTimeout(take.limitTimer)
    const heldMs = performance.now() - take.startedAt

    const blob = await new Promise<Blob>((resolve) => {
      take.rtc.stopRecording(() => resolve(take.rtc.getBlob()))
    })
    take.stream.getTracks().forEach((track) => track.stop())
    take.rtc.destroy()

    return heldMs < MIN_MS ? null : blob.arrayBuffer()
  }

  private async open(): Promise<Take> {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
    // These options make RecordRTC produce exactly what the backend accepts:
    // mono, 16 kHz, 16-bit PCM WAV.
    const rtc = new RecordRTC(stream, {
      type: 'audio',
      mimeType: 'audio/wav',
      recorderType: RecordRTC.StereoAudioRecorder,
      desiredSampRate: SAMPLE_RATE,
      numberOfAudioChannels: 1,
    })
    rtc.startRecording()
    return {
      stream,
      rtc,
      startedAt: performance.now(),
      limitTimer: window.setTimeout(() => this.onLimit?.(), LIMIT_MS),
    }
  }
}
