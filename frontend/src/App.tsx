import { ChevronDown } from 'lucide-react'
import { Avatar } from '@/components/Avatar'
import { ChatView } from '@/components/chat/ChatView'
import { ChatsView } from '@/components/chats/ChatsView'
import { PeopleView } from '@/components/people/PeopleView'
import { NameDialog } from '@/components/profile/NameDialog'
import { setNameDialogOpen, setView, useSession } from '@/store/session'

const views = { people: PeopleView, chat: ChatView, chats: ChatsView }

export default function App() {
  const { view, status, name } = useSession()
  const View = views[view]
  const online = status === 'open'
  return (
    <div className="mx-auto flex min-h-svh max-w-[1680px] flex-col overflow-x-clip bg-[radial-gradient(ellipse_at_66%_43%,#42443b32,transparent_49%)] px-5 sm:px-7 lg:px-12">
      <header className="relative z-10 flex h-20 shrink-0 items-center justify-between gap-3 border-b border-border sm:h-[99px]">
        <nav aria-label="Main navigation" className="flex gap-5 sm:gap-8">
          {(['people', 'chats'] as const).map((tab) => {
            const active = view === tab || (tab === 'people' && view === 'chat')
            return (
              <button
                key={tab}
                aria-current={active ? 'page' : undefined}
                onClick={() => setView(tab)}
                className={`flex items-center gap-2 py-3 text-xs transition-colors hover:text-foreground ${active ? 'text-foreground' : 'text-muted'}`}
              >
                <span
                  aria-hidden="true"
                  className={`size-1.5 rounded-full bg-primary ${active ? '' : 'invisible'}`}
                />
                {tab === 'people' ? 'People' : 'Chats'}
              </button>
            )
          })}
        </nav>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setNameDialogOpen(true)}
            className="flex items-center gap-2 text-xs"
            aria-label={name ? `Edit username: ${name}` : 'Set username'}
          >
            {name ? (
              <>
                <Avatar name={name} online={online} className="size-8 text-xs" />
                <span className="hidden max-w-32 truncate sm:block">{name}</span>
                <ChevronDown className="hidden size-4 sm:block" />
              </>
            ) : (
              'Set username'
            )}
          </button>
          <span
            role="status"
            className={`flex items-center gap-2 rounded-full border px-3 py-2 text-[10px] sm:text-xs ${online ? 'border-online/20 bg-online/5 text-online' : 'border-offline/25 bg-offline/5 text-offline'}`}
          >
            <span
              aria-hidden="true"
              className={`size-2 rounded-full ${online ? 'bg-online' : 'bg-offline'}`}
            />
            {online ? 'Online' : 'Reconnecting…'}
          </span>
        </div>
      </header>
      <main className="flex-1">
        <View />
      </main>
      <footer className="flex justify-between gap-4 py-5 text-[10px] text-muted">
        <span>Audio & transcripts</span>
        <span>Chats stay until you reload</span>
      </footer>
      <NameDialog />
    </div>
  )
}
