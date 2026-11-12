import { Avatar } from '@/components/Avatar'
import { Button } from '@/components/ui/button'
import { ChatView } from '@/features/chat/ChatView'
import { ChatsView } from '@/features/chats/ChatsView'
import { PeopleView } from '@/features/people/PeopleView'
import { NameDialog } from '@/features/profile/NameDialog'
import { setNameDialogOpen, setView, useSession } from '@/store/session'

const views = { people: PeopleView, chat: ChatView, chats: ChatsView }

export default function App() {
  const view = useSession((s) => s.view)
  const status = useSession((s) => s.status)
  const name = useSession((s) => s.name)
  const View = views[view]
  const onChats = view === 'chats'

  return (
    <div className="mx-auto flex min-h-svh max-w-3xl flex-col gap-8 p-4">
      <header className="flex items-center gap-2">
        <nav aria-label="Main navigation" className="flex gap-1">
          <Button
            variant={onChats ? 'ghost' : 'secondary'}
            aria-current={onChats ? undefined : 'page'}
            onClick={() => setView('people')}
          >
            People
          </Button>
          <Button
            variant={onChats ? 'secondary' : 'ghost'}
            aria-current={onChats ? 'page' : undefined}
            onClick={() => setView('chats')}
          >
            Chats
          </Button>
        </nav>
        <span className="ml-auto text-sm text-muted-foreground" role="status">
          {status === 'open' ? '' : 'Reconnecting…'}
        </span>
        <Button variant="ghost" onClick={() => setNameDialogOpen(true)}>
          {name ? (
            <>
              <Avatar name={name} className="size-6 text-xs" />
              {name}
            </>
          ) : (
            'Set username'
          )}
        </Button>
      </header>

      <main>
        <View />
      </main>

      <NameDialog />
    </div>
  )
}

