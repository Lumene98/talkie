import { Mic } from 'lucide-react'
import { useHoldToRecord } from '@/hooks/useHoldToRecord'
import { cn } from '@/lib/utils'
import type { UserId } from '@/protocol/types'
import { useSession } from '@/store/session'
import { failureText, startTalk, stopTalk, useUpload } from '@/store/upload'

export function RecordButton({ peerId, peerOnline }: { peerId: UserId; peerOnline: boolean }) {
  const upload = useUpload()
  const connected = useSession((s) => s.status === 'open')
  const available = peerOnline && connected
  const recording = upload.phase === 'recording'

  const handlers = useHoldToRecord(
    available,
    () => void startTalk(peerId),
    () => void stopTalk(),
  )

  let label = 'Hold to talk'
  let hint = 'Hold Space or this button'
  if (!peerOnline) {
    label = 'They left'
    hint = 'Messages stay until you reload'
  } else if (!connected) {
    label = 'Reconnecting…'
    hint = ''
  } else if (recording) {
    label = 'Recording'
    hint = 'Release to send'
  } else if (upload.phase === 'requesting' || upload.phase === 'uploading') {
    label = 'Sending…'
    hint = ''
  } else if (upload.phase === 'failed') {
    hint = failureText(upload.code)
  }

  return (
    <button
      type="button"
      disabled={!available}
      aria-pressed={recording}
      {...handlers}
      className={cn(
        'mx-auto flex size-40 touch-none flex-col items-center justify-center gap-1 rounded-full border border-border select-none disabled:opacity-50',
        recording && 'border-primary bg-primary text-primary-foreground',
      )}
    >
      <Mic aria-hidden="true" />
      <strong>{label}</strong>
      <small className="px-4 text-center text-xs opacity-70">{hint}</small>
    </button>
  )
}

