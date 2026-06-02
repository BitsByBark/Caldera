browser.runtime.onMessage.addListener((message) => {
  if (message.type !== 'CALDERA_DOWNLOAD') return undefined

  console.log('CALDERA download request', message)

  return fetch('http://127.69.67.21:7337/download', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      url: message.url,
      game_domain: message.game_domain,
      mod_id: message.mod_id,
      file_id: message.file_id
    })
  })
    .then((res) => {
      if (!res.ok) throw new Error(`CALDERA returned ${res.status}`)
      console.log('CALDERA queued download')
      return { success: true }
    })
    .catch((err) => {
      console.error('CALDERA extension error:', err)
      return { success: false, error: err.message }
    })
})
