import { addMessage, clearChats, setTranscript } from '@/store/chats'
import { useSession, userName } from '@/store/session'
import { handleAccepted, handleDisconnect, handleError, handleUploadReady } from '@/store/upload'
import { socket } from './client'

/** Routes socket events into the stores. Call once at startup. */
export function startClient(): void {
  socket.handlers = {
    onStatus(status) {
      useSession.setState({ status })
      if (status === 'closed') handleDisconnect()
    },

    onMessage(message) {
      switch (message.type) {
        case 'welcome': {
          // New connection = new identity; peers cannot reach the old one.
          clearChats()
          useSession.setState({
            selfId: message.user_id,
            activePeerId: null,
            view: 'people',
          })
          const { name } = useSession.getState()
          if (name) socket.send({ type: 'set_name', name })
          break
        }
        case 'users':
          useSession.setState({ users: message.users })
          break
        case 'upload_ready':
          handleUploadReady(message.request_id, message.message_id)
          break
        case 'message_accepted':
          handleAccepted(message.message_id)
          break
        case 'transcript_ready':
          setTranscript(message.message_id, {
            status: 'ready',
            text: message.transcript_body,
          })
          break
        case 'transcript_failed':
          setTranscript(message.message_id, { status: 'failed' })
          break
        case 'error':
          console.warn(`server error: ${message.code}: ${message.message}`)
          handleError(message.code)
          break
      }
    },

    onAudio(meta, wav) {
      addMessage({
        id: meta.message_id,
        peerId: meta.sender_id,
        peerName: userName(meta.sender_id),
        mine: false,
        wav,
        at: Date.now(),
        transcript: { status: 'pending' },
      })
    },
  }
  socket.connect()
}
