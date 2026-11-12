import { Avatar } from '@/components/Avatar'
import { formatTime } from '@/lib/format'
import { transcriptText, useChats } from '@/store/chats'
import { openChat } from '@/store/session'

export function ChatsView() {
  const byPeer = useChats((s) => s.byPeer)
  const latest = Object.values(byPeer)
    .map((messages) => messages[messages.length - 1])
    .sort((a, b) => b.at - a.at)

  if (latest.length === 0) {
    return (
      <p className="text-muted-foreground">
        No chats yet. Select a user in People to start a conversation.
      </p>
    )
  }

  return (
    <section aria-label="Chats">
      <ul className="flex flex-col gap-2">
        {latest.map((last) => (
          <li key={last.peerId}>
            <button
              type="button"
              onClick={() => openChat(last.peerId)}
              className="flex w-full items-center gap-3 rounded-2xl border border-border p-3 text-left hover:bg-secondary"
            >
              <Avatar name={last.peerName} />
              <span className="min-w-0 flex-1">
                <strong className="block truncate">{last.peerName}</strong>
                <span className="block truncate text-sm text-muted-foreground">
                  {last.mine ? 'You' : last.peerName}: {transcriptText(last.transcript)}
                </span>
              </span>
              <span className="text-xs text-muted-foreground">{formatTime(last.at)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  )
}
