import { useEffect, useRef } from 'react'
import { ArrowLeft, Play } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { formatTime } from '@/lib/format'
import { cn } from '@/lib/utils'
import { player } from '@/net/client'
import { transcriptText, useChats } from '@/store/chats'
import type { ChatMessage } from '@/store/chats'
import { setView, useSession } from '@/store/session'
import { stopTalk } from '@/store/upload'
import { RecordButton } from './RecordButton'

const NO_MESSAGES: ChatMessage[] = []

export function ChatView() {
  const peerId = useSession((s) => s.activePeerId)
  const peer = useSession((s) => s.users.find((user) => user.id === s.activePeerId))
  const messages = useChats((s) => (peerId ? s.byPeer[peerId] : undefined)) ?? NO_MESSAGES
  const bottom = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottom.current?.scrollIntoView({ block: 'end' })
  }, [messages.length])

  useEffect(
    () => () => {
      player.stop()
      void stopTalk()
    },
    [],
  )

  if (!peerId) return null
  const peerName = peer?.name ?? messages.at(-1)?.peerName ?? 'Unknown'

  return (
    <section aria-label={`Chat with ${peerName}`} className="flex flex-col gap-6">
      <div className="flex items-center gap-3">
        <Button variant="ghost" size="icon" aria-label="Back to people" onClick={() => setView('people')}>
          <ArrowLeft />
        </Button>
        <h1 className="font-heading text-2xl">{peerName}</h1>
        <span className="text-sm text-muted-foreground">{peer ? 'Online' : 'Left'}</span>
      </div>

      <ol className="flex flex-col gap-3">
        {messages.length === 0 && <li className="text-muted-foreground">No messages yet.</li>}
        {messages.map((message) => (
          <li
            key={message.id}
            className={cn(
              'max-w-[80%] rounded-2xl border border-border p-3',
              message.mine && 'self-end bg-secondary',
            )}
          >
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <strong>{message.mine ? 'You' : message.peerName}</strong>
              <span>{formatTime(message.at)}</span>
            </div>
            <div className="mt-2 flex items-start gap-2">
              <Button
                variant="outline"
                size="icon"
                aria-label="Play message"
                onClick={() => player.play(message.wav)}
              >
                <Play />
              </Button>
              <p className={cn(message.transcript.status !== 'ready' && 'text-muted-foreground', )}>
                {transcriptText(message.transcript)}
              </p>
            </div>
          </li>
        ))}
      </ol>
      <div ref={bottom} />

      <RecordButton peerId={peerId} peerOnline={peer !== undefined} />
    </section>
  )
}

