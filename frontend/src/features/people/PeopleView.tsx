import { Avatar } from '@/components/Avatar'
import { openChat, setNameDialogOpen, useSession } from '@/store/session'

export function PeopleView() {
  const users = useSession((s) => s.users)
  const selfId = useSession((s) => s.selfId)
  const hasName = useSession((s) => s.name !== '')
  const peers = users.filter((user) => user.id !== selfId)

  if (peers.length === 0) {
    return <p className="text-muted-foreground">No one else is here yet.</p>
  }

  return (
    <section aria-label="People">
      <p className="mb-4 text-muted-foreground">Select a user to open a chat.</p>
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-3">
        {peers.map((user) => (
          <li key={user.id}>
            <button
              type="button"
              onClick={() => (hasName ? openChat(user.id) : setNameDialogOpen(true))}
              className="flex w-full flex-col items-center gap-2 rounded-2xl border border-border p-4 hover:bg-secondary"
            >
              <Avatar name={user.name} className="size-16 text-xl" />
              <span className="max-w-full truncate">{user.name}</span>
              <span className="text-xs text-muted-foreground">
                {user.busy ? 'Sending…' : 'Online'}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  )
}
