import { ChatView } from '@/features/chat/ChatView'
import { ChatsView } from '@/features/chats/ChatsView'
import { PeopleView } from '@/features/people/PeopleView'
import { NameDialog } from '@/features/profile/NameDialog'
import { useSession } from '@/store/session'

const views = { people: PeopleView, chat: ChatView, chats: ChatsView }

export default function App() {
  const View = views[useSession((s) => s.view)]
  return (
    <>
      {/* TODO: header with People / Chats nav and profile. */}
      <main>
        <View />
      </main>
      <NameDialog />
    </>
  )
}
