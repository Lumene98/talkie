export function initial(name: string): string {
  return [...name.trim()][0]?.toUpperCase() ?? '?'
}

export function formatTime(at: number): string {
  return new Date(at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}
