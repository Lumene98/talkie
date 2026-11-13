import { Mic } from 'lucide-react'
import { useHoldToRecord } from '@/hooks/useHoldToRecord'
import { cn } from 'cn'
import { getRecordButtonLabels } from '@/lib/utils'
import type { UserId } from '@/protocol/types'
import { useSession } from '@/store/session'
import { startTalk, stopTalk, useUpload } from '@/store/upload'

export function RecordButton({ peerId, peerOnline }: { peerId: UserId; peerOnline: boolean }) {
  const upload = useUpload()
  const connected = useSession((s) => s.status === 'open')
  const nameDialogOpen = useSession((s) => s.nameDialogOpen)
  const available = peerOnline && connected && !nameDialogOpen
  const recording = upload.phase === 'recording'

  const handlers = useHoldToRecord(
    available,
    () => void startTalk(peerId),
    () => void stopTalk(),
  )

  const { label, hint } = getRecordButtonLabels(peerOnline, connected, upload)

  return (
    <button
      type="button"
      data-hold-to-talk
      disabled={!available}
      aria-pressed={recording}
      {...handlers}
      className={cn(
        'mx-auto flex size-56 touch-none flex-col items-center justify-center gap-3 rounded-full bg-foreground text-background shadow-[0_0_0_13px_#ffffff03,0_0_0_14px_#ffffff10] select-none disabled:bg-offline disabled:opacity-70 motion-safe:transition-all',
        recording &&
          'bg-primary text-primary-foreground shadow-[0_0_0_17px_#ff693617,0_0_0_35px_#ff693607]',
      )}
    >
      <Mic aria-hidden="true" className="size-8" strokeWidth={1.3} />
      <strong className="text-xl font-medium tracking-tight">{label}</strong>
      <small className="px-4 text-center text-xs opacity-70">{hint}</small>
    </button>
  )
}
