import { writable } from 'svelte/store'

export const sessionLog = writable([])

export function addLog(message, type = 'default', extra = {}) {
  const now = new Date()
  const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`
  sessionLog.update((log) => [...log, { time, message, type, ...extra }])
}

export function upsertProgressLog(key, message, progress = 0, type = 'info') {
  const now = new Date()
  const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`
  sessionLog.update((log) => {
    const idx = log.findIndex((entry) => entry.progressKey === key)
    const next = { time, message, type, progress, progressKey: key }
    if (idx === -1) return [...log, next]
    const copy = [...log]
    copy[idx] = next
    return copy
  })
}
