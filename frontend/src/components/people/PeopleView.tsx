import { useState } from 'react'
import { ArrowUpRight } from 'lucide-react'
import { Avatar } from '@/components/Avatar'
import { useChats } from '@/store/chats'
import { openChat, setNameDialogOpen, useSession } from '@/store/session'
import { formatUsers } from '@/lib/utils'

const positions = [
  'left-[17%] top-[2%]',
  'right-[7%] top-[12%]',
  'left-0 top-[51%]',
  'right-[11%] top-[70%]',
]
const sizes = [
  'size-24 sm:size-36',
  'size-28 sm:size-44',
  'size-24 sm:size-40',
  'size-20 sm:size-28',
]
const colors = ['bg-[#b8b39d]', 'bg-[#acb6a0]', 'bg-[#c88d70]', 'bg-[#a7a4ba]']

export function PeopleView() {
  const { users, selfId, name, status } = useSession()
  const byPeer = useChats((s) => s.byPeer)
  const [filter, setFilter] = useState<'all' | 'online'>('all')
  const [hovered, setHovered] = useState<string | null>(null)
  const peers = formatUsers(users, status, selfId, byPeer)
  const visible = peers.filter((u) => filter === 'all' || u.online)
  const selected = visible.find((u) => u.id === hovered) ?? visible[0]
  function enter(id: string) {
    if (name) openChat(id)
    else setNameDialogOpen(true)
  }

  return (
    <section aria-label="People">
      <div className="flex items-center justify-between gap-4 pt-7 text-xs text-muted">
        <p>Select a user to open a chat.</p>
        <span className="flex items-center gap-2 whitespace-nowrap">
          <span className="size-2 rounded-full bg-online" />
          {peers.filter((u) => u.online).length} online
        </span>
      </div>
      <div className="relative isolate min-h-[510px] lg:min-h-[650px]">
        <div aria-hidden="true" className="pointer-events-none absolute inset-0 overflow-hidden">
          {[
            'size-[330px] sm:size-[535px]',
            'size-[490px] sm:size-[790px]',
            'size-[650px] sm:size-[1060px]',
          ].map((size) => (
            <div
              key={size}
              className={`absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full border border-orbit ${size}`}
            />
          ))}
        </div>
        <div className="absolute bottom-0 left-1/2 z-10 flex -translate-x-1/2 gap-5 lg:top-1/2 lg:bottom-auto lg:left-0 lg:translate-x-0 lg:-translate-y-1/2 lg:flex-col">
          <span className="hidden text-[10px] tracking-[.2em] text-muted lg:block">USERS</span>
          {(['all', 'online'] as const).map((value) => (
            <button
              key={value}
              onClick={() => setFilter(value)}
              aria-pressed={filter === value}
              className={`flex items-center gap-3 text-xs ${filter === value ? 'text-foreground' : 'text-muted'}`}
            >
              <span
                className={`size-1 rounded-full ${filter === value ? 'bg-primary' : 'bg-transparent'}`}
              />
              {value === 'all' ? 'All users' : 'Online'}
            </button>
          ))}
        </div>
        <div className="relative mx-auto h-[470px] w-full max-w-[760px] sm:h-[560px] lg:top-8 lg:w-[68%]">
          <div className="absolute top-1/2 left-1/2 flex size-32 -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center rounded-full bg-foreground text-background sm:size-60">
            <span className="text-[7px] tracking-[.18em] text-subtle sm:text-[9px]">
              DIRECT MESSAGES
            </span>
            <h1 className="my-3 text-center text-2xl leading-tight tracking-tight sm:my-5 sm:text-3xl">
              Select
              <br />a user
            </h1>
            <button
              disabled={!selected}
              onClick={() => selected && enter(selected.id)}
              aria-label={selected ? `Open chat with ${selected.name}` : 'No users available'}
              className="grid size-8 place-items-center rounded-full border border-black/20"
            >
              <ArrowUpRight className="size-4" />
            </button>
          </div>
          {visible.slice(0, 4).map((user, i) => (
            <button
              key={user.id}
              onMouseEnter={() => setHovered(user.id)}
              onFocus={() => setHovered(user.id)}
              onClick={() => enter(user.id)}
              className={`absolute flex flex-col items-center gap-2 rounded-full motion-safe:transition-transform motion-safe:hover:-translate-y-1 ${positions[i]}`}
              aria-label={`Chat with ${user.name}, ${user.online ? 'online' : 'offline'}`}
            >
              <Avatar
                name={user.name}
                online={user.online}
                className={`${sizes[i]} ${colors[i]} text-4xl sm:text-6xl`}
              />
              <span className="max-w-28 truncate text-sm sm:max-w-44">{user.name}</span>
              <span
                className={`text-[10px] sm:text-xs ${user.online ? 'text-online' : 'text-offline'}`}
              >
                {user.online ? (user.busy ? 'Sending…' : 'Online') : 'Offline'}
              </span>
            </button>
          ))}
        </div>
        <aside className="absolute top-1/2 right-0 hidden w-36 -translate-y-1/2 xl:block">
          <div className="mb-5 h-px w-7 bg-primary" />
          <span className="text-[10px] tracking-[.2em] text-muted">
            {selected ? (selected.online ? 'ONLINE' : 'OFFLINE') : 'PEOPLE'}
          </span>
          <h2 className="mt-3 truncate text-2xl tracking-tight">
            {selected?.name ?? 'No one here yet'}
          </h2>
          <p className="my-4 text-xs leading-6 text-muted">
            {selected
              ? selected.online
                ? 'Available to chat'
                : 'View your conversation'
              : 'Open another browser to connect.'}
          </p>
          {selected && (
            <button
              onClick={() => enter(selected.id)}
              className="flex items-center gap-5 border-b border-border-strong pb-2 text-xs"
            >
              Open chat <ArrowUpRight className="size-4" />
            </button>
          )}
        </aside>
      </div>
      {!visible.length && (
        <p className="py-6 text-center text-xs text-muted">
          No one else is here yet. Open another browser to connect.
        </p>
      )}
      {visible.length > 4 && (
        <ul className="flex flex-wrap justify-center gap-6 py-8">
          {visible.slice(4).map((user) => (
            <li key={user.id}>
              <button onClick={() => enter(user.id)} className="flex items-center gap-3 text-sm">
                <Avatar name={user.name} online={user.online} />
                {user.name}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
