import { useEffect, useMemo, useRef } from 'react'
import type { PointerEvent } from 'react'

/**
 * Press-and-hold by pointer or Space. `onStart` and `onStop` fire once per hold,
 * however many events end it (release, pointer lost, window blur, …).
 */
export function useHoldToRecord(enabled: boolean, onStart: () => void, onStop: () => void) {
  const held = useRef(false)
  const latest = useRef({ enabled, onStart, onStop })
  useEffect(() => {
    latest.current = { enabled, onStart, onStop }
  })

  const hold = useMemo(
    () => ({
      begin() {
        if (held.current || !latest.current.enabled) return
        held.current = true
        latest.current.onStart()
      },
      end() {
        if (!held.current) return
        held.current = false
        latest.current.onStop()
      },
    }),
    [],
  )

  useEffect(() => {
    const isTyping = (target: EventTarget | null) =>
      target instanceof HTMLElement && ['INPUT', 'TEXTAREA'].includes(target.tagName)

    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.code !== 'Space' ||
        isTyping(event.target) ||
        document.querySelector('dialog[open]')
      )
        return
      if (
        event.target instanceof HTMLElement &&
        event.target.closest('button, a, audio, select') &&
        !event.target.closest('[data-hold-to-talk]')
      )
        return
      event.preventDefault()
      if (!event.repeat) hold.begin()
    }
    const onKeyUp = (event: KeyboardEvent) => {
      if (event.code === 'Space') hold.end()
    }

    document.addEventListener('keydown', onKeyDown)
    document.addEventListener('keyup', onKeyUp)
    window.addEventListener('blur', hold.end)
    return () => {
      document.removeEventListener('keydown', onKeyDown)
      document.removeEventListener('keyup', onKeyUp)
      window.removeEventListener('blur', hold.end)
      hold.end()
    }
  }, [hold])

  return {
    onPointerDown(event: PointerEvent<HTMLButtonElement>) {
      if (event.button !== 0) return
      // Keep receiving pointer events even if the pointer leaves the button.
      event.currentTarget.setPointerCapture(event.pointerId)
      hold.begin()
    },
    onPointerUp: hold.end,
    onPointerCancel: hold.end,
    onLostPointerCapture: hold.end,
  }
}
