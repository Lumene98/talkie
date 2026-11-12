export class Player {
  private audio: HTMLAudioElement | null = null
  private url: string | null = null

  play(wav: ArrayBuffer): void {
    this.stop()
    this.url = URL.createObjectURL(new Blob([wav], { type: 'audio/wav' }))
    this.audio = new Audio(this.url)
    this.audio.onended = () => this.stop()
    void this.audio.play()
  }

  stop(): void {
    this.audio?.pause()
    this.audio = null
    if (this.url) URL.revokeObjectURL(this.url)
    this.url = null
  }
}
