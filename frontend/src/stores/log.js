import { writable } from 'svelte/store'

export const sessionLog = writable([])

export function addLog(message, type = 'default') {
  const now = new Date()
  const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`
  sessionLog.update((log) => [...log, { time, message, type }])
}
