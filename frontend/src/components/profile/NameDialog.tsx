import { useEffect, useRef, useState } from 'react'
import type { FormEvent } from 'react'
import { ArrowRight, X } from 'lucide-react'
import { NAME_MAX_CHARS } from '@/protocol/types'
import { setName, setNameDialogOpen, useSession } from '@/store/session'

export function NameDialog() {
  const open = useSession((s) => s.nameDialogOpen)
  const current = useSession((s) => s.name)
  const dialog = useRef<HTMLDialogElement>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (open) dialog.current?.showModal()
    else dialog.current?.close()
  }, [open])

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const value = new FormData(event.currentTarget).get('name')
    setError(setName(typeof value === 'string' ? value : ''))
  }

  return (
    <dialog
      ref={dialog}
      onCancel={() => setNameDialogOpen(false)}
      onClose={() => {
        setError(null)
        setNameDialogOpen(false)
      }}
      aria-labelledby="name-title"
      className="fixed inset-0 m-auto w-[420px] max-w-[calc(100%-32px)] rounded-3xl border border-border-strong bg-surface p-8 text-foreground shadow-2xl backdrop:bg-black/70 backdrop:backdrop-blur-sm"
    >
      <div className="mb-7 flex items-center justify-between">
        <span aria-hidden="true" className="text-3xl text-primary">
          ✳
        </span>
        <button
          onClick={() => setNameDialogOpen(false)}
          aria-label="Close"
          className="grid size-8 place-items-center rounded-full border border-border"
        >
          <X className="size-4" />
        </button>
      </div>
      <h2 id="name-title" className="text-3xl tracking-tight">
        Set username
      </h2>
      <p className="my-5 text-xs leading-6 text-muted">Your name appears on messages you send.</p>
      {open && (
        <form onSubmit={submit}>
          <label htmlFor="username" className="mb-2 block text-xs text-muted">
            Username
          </label>
          <input
            id="username"
            name="name"
            defaultValue={current}
            maxLength={NAME_MAX_CHARS}
            autoComplete="nickname"
            required
            autoFocus
            aria-invalid={error !== null}
            aria-describedby={error ? 'name-error' : undefined}
            placeholder="e.g. Luca"
            className="mb-5 w-full rounded-lg border border-border-strong bg-background p-4 text-sm"
          />
          {error && (
            <p id="name-error" role="alert" className="mb-4 text-xs text-primary">
              {error}
            </p>
          )}
          <button
            type="submit"
            className="flex w-full items-center justify-center gap-3 rounded-full bg-primary p-4 text-sm text-background"
          >
            Continue <ArrowRight className="size-4" />
          </button>
        </form>
      )}
    </dialog>
  )
}
