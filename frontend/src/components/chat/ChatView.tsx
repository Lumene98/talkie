import { useEffect, useRef } from 'react'
import { ArrowLeft, ArrowUpRight } from 'lucide-react'
import { Avatar } from '@/components/Avatar'
import { formatTime } from '@/lib/format'
import { player } from '@/net/client'
import { transcriptText, useChats } from '@/store/chats'
import type { ChatMessage } from '@/store/chats'
import { setView, useSession } from '@/store/session'
import { stopTalk } from '@/store/upload'
import { RecordButton } from './RecordButton'

const NO_MESSAGES: ChatMessage[] = []

function AudioMessage({ wav }: { wav: ArrayBuffer }) {
  const audio = useRef<HTMLAudioElement>(null)
  useEffect(() => {
    const value = URL.createObjectURL(new Blob([wav], { type: 'audio/wav' }))
    const element = audio.current
    if (element) element.src = value
    return () => {
      element?.pause()
      element?.removeAttribute('src')
      URL.revokeObjectURL(value)
    }
  }, [wav])
  return (
    <audio
      controls
      preload="metadata"
      ref={audio}
      aria-label="Voice message"
      className="my-3 h-10 w-full min-w-0 rounded-lg"
    />
  )
}

export function ChatView() {
  const { activePeerId: peerId, users, name, status } = useSession()
  const peer = users.find((u) => u.id === peerId)
  const messages = useChats((s) => (peerId ? s.byPeer[peerId] : undefined)) ?? NO_MESSAGES
  const list = useRef<HTMLDivElement>(null)
  const online = !!peer && status === 'open'

  useEffect(() => {
    if (list.current) list.current.scrollTop = list.current.scrollHeight
  }, [messages])

  useEffect(
    () => () => {
      player.stop()
      void stopTalk()
    },
    [peerId],
  )

  if (!peerId) return null
  const peerName = peer?.name ?? messages.at(-1)?.peerName ?? 'Unknown'

  return (
    <section aria-label={`Chat with ${peerName}`} className="pt-8">
      <div className="flex items-center justify-between gap-4 text-xs">
        <div className="flex min-w-0 items-center gap-3">
          <button onClick={() => setView('people')} className="flex items-center gap-2">
            <ArrowLeft className="size-4" />
            People
          </button>
          <span className="text-muted">/</span>
          <span className="truncate text-muted">{peerName}</span>
        </div>
        <button
          onClick={() => setView('people')}
          className="flex shrink-0 items-center gap-3 rounded-full border border-border px-4 py-3 hover:bg-surface"
        >
          Close chat <ArrowUpRight className="size-4" />
        </button>
      </div>
      <div className="grid gap-8 pt-8 lg:grid-cols-[1fr_350px] lg:gap-12">
        <div className="relative isolate min-h-[510px] overflow-hidden text-center">
          <div
            aria-hidden="true"
            className="pointer-events-none absolute top-[55%] left-1/2 -z-10 size-[390px] -translate-x-1/2 -translate-y-1/2 rounded-full border border-orbit sm:size-[510px]"
          />
          <span
            className={`text-[10px] tracking-[.2em] ${online ? 'text-online' : 'text-offline'}`}
          >
            {online ? 'ONLINE' : 'OFFLINE'}
          </span>
          <h1 className="mt-3 text-4xl tracking-tight break-words">{peerName}</h1>
          <p className="mt-3 text-xs text-muted">
            {online ? 'Available to chat' : 'You can still listen to previous messages'}
          </p>
          <div className="my-7 flex items-center justify-center gap-4 text-xs">
            <span className="flex items-center gap-2">
              <Avatar name={name || 'You'} className="size-7 text-[10px]" />
              You
            </span>
            <span aria-hidden="true" className="w-12 border-t border-dashed border-subtle" />
            <span className="flex max-w-[50%] items-center gap-2">
              <Avatar name={peerName} className="size-7 bg-[#acb6a0] text-[10px]" />
              <span className="truncate">{peerName}</span>
            </span>
          </div>
          <RecordButton peerId={peerId} peerOnline={online} />
          <p className="mt-7 text-[10px] text-muted">Release to send · up to 15 seconds</p>
        </div>
        <aside className="flex h-[530px] min-w-0 flex-col overflow-hidden rounded-2xl border border-border bg-white/[.015]">
          <div className="flex shrink-0 items-center justify-between border-b border-border px-5 py-5">
            <h2 className="text-sm">Messages</h2>
          </div>
          <div
            ref={list}
            role="log"
            aria-label="Conversation"
            className="min-h-0 flex-1 space-y-6 overflow-y-auto overscroll-contain p-5"
          >
            {!messages.length && (
              <p className="py-6 text-center text-xs text-muted">No messages yet. Hold to talk.</p>
            )}
            {messages.map((message) => (
              <article key={message.id} className="border-b border-border pb-5 last:border-0">
                <div className="flex items-center justify-between gap-3 text-[10px]">
                  <strong className={message.mine ? 'text-primary' : 'text-foreground'}>
                    {message.mine ? 'You' : message.peerName}
                  </strong>
                  <time className="shrink-0 text-muted">{formatTime(message.at)}</time>
                </div>
                <AudioMessage wav={message.wav} />
                <p
                  className={`text-xs leading-7 break-words ${message.transcript.status === 'ready' ? 'text-foreground' : 'text-muted'}`}
                >
                  {transcriptText(message.transcript)}
                </p>
              </article>
            ))}
          </div>
          <div className="flex shrink-0 items-center justify-between border-t border-border px-5 py-4 text-[10px] text-muted">
            <span>Transcripts shown below audio</span>
          </div>
        </aside>
      </div>
    </section>
  )
}
