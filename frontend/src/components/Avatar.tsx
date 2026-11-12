import { initial } from '@/lib/format'
import { cn } from '@/lib/utils'

export function Avatar({ name, className }: { name: string; className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cn(
        'flex size-10 shrink-0 items-center justify-center rounded-full bg-secondary font-heading font-semibold',
        className,
      )}
    >
      {initial(name)}
    </span>
  )
}
