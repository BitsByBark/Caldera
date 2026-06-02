chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type !== 'CALDERA_DOWNLOAD') return

  fetch('http://localhost:7337/download', {
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
      sendResponse({ success: true })
    })
    .catch((err) => {
      console.error('CALDERA extension error:', err)
      sendResponse({ success: false, error: err.message })
    })

  return true
})
