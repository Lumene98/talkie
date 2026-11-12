import { useState } from 'react'
import type { FormEvent } from 'react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { NAME_MAX_CHARS } from '@/protocol/types'
import { setName, setNameDialogOpen, useSession } from '@/store/session'

export function NameDialog() {
  const open = useSession((s) => s.nameDialogOpen)
  console.log(open)

  const current = useSession((s) => s.name)
  const [error, setError] = useState<string | null>(null)

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const value = new FormData(event.currentTarget).get('name')
    setError(setName(typeof value === 'string' ? value : ''))
  }

  return (
    <Dialog open={open} onOpenChange={setNameDialogOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Set username</DialogTitle>
          <DialogDescription>Your name appears on messages you send.</DialogDescription>
        </DialogHeader>
        <form onSubmit={submit} className="flex flex-col gap-3">
          <Input
            name="name"
            defaultValue={current}
            maxLength={NAME_MAX_CHARS}
            autoComplete="nickname"
            aria-label="Username"
            aria-invalid={error !== null}
            autoFocus
          />
          {error && <p className="text-sm text-destructive">{error}</p>}
          <Button type="submit">Continue</Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
