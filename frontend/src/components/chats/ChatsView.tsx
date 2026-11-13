import { Avatar } from '@/components/Avatar'
import { formatTime } from '@/lib/format'
import { transcriptText, useChats } from '@/store/chats'
import { openChat, useSession } from '@/store/session'

export function ChatsView() {
  const users = useSession((s) => s.users)
  const connected = useSession((s) => s.status === 'open')
  const byPeer = useChats((s) => s.byPeer)
  const latest = Object.values(byPeer)
    .map((messages) => messages[messages.length - 1])
    .sort((a, b) => b.at - a.at)

  if (latest.length === 0) {
    return (
      <p className="py-14 text-sm text-muted">
        No chats yet. Select a user in People to start a conversation.
      </p>
    )
  }

  return (
    <section aria-label="Chats" className="min-h-[650px] py-12">
      <h1 className="text-2xl tracking-tight">Chats</h1>
      <p className="mt-3 text-xs text-muted">Your conversations from this session.</p>
      <ul className="mt-9 flex max-w-[850px] flex-col border-b border-border">
        {latest.map((last) => (
          <li key={last.peerId}>
            <button
              type="button"
              onClick={() => openChat(last.peerId)}
              className="flex w-full items-center gap-4 border-t border-border px-3 py-6 text-left hover:bg-white/[.025]"
            >
              <Avatar
                name={last.peerName}
                online={connected && users.some((u) => u.id === last.peerId)}
              />
              <span className="min-w-0 flex-1">
                <strong className="block truncate font-medium">{last.peerName}</strong>
                <span className="mt-2 block truncate text-xs text-muted">
                  {last.mine ? 'You' : last.peerName}: {transcriptText(last.transcript)}
                </span>
              </span>
              <span className="text-xs text-muted">{formatTime(last.at)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  )
}
