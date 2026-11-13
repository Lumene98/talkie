import { cn } from 'cn'
import { initial } from '@/lib/format'

export function Avatar({
  name,
  online,
  selected = false,
  className,
}: {
  name: string
  online?: boolean
  selected?: boolean
  className?: string
}) {
  return (
    <span
      className={cn(
        'relative inline-flex size-10 shrink-0 items-center justify-center rounded-full bg-[#b8b39d] font-medium text-[#252820]',
        selected && 'ring-1 ring-foreground ring-offset-8 ring-offset-background',
        className,
      )}
    >
      <span aria-hidden="true">{initial(name)}</span>
      {online !== undefined && (
        <span
          aria-label={online ? 'Online' : 'Offline'}
          className={cn(
            'absolute right-0 bottom-0 size-[.28em] min-h-3 min-w-3 rounded-full border-[3px] border-background',
            online ? 'bg-online' : 'bg-offline',
          )}
        />
      )}
    </span>
  )
}
