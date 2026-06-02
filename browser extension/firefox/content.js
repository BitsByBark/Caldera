function injectButtons() {
  const manualButtons = document.querySelectorAll(
    'a[class*="btn-manual"], a.nxm-button[href*="file_id="]'
  )

  manualButtons.forEach((btn) => {
    if (btn.nextElementSibling?.classList?.contains('caldera-dl-btn')) return

    const fileUrl = btn.href
    if (!fileUrl) return

    const calderaBtn = document.createElement('a')
    calderaBtn.className = 'caldera-dl-btn'
    calderaBtn.textContent = '⬇ CALDERA'
    calderaBtn.dataset.url = fileUrl
    calderaBtn.style.cssText = `
      display: inline-block;
      margin-left: 6px;
      padding: 4px 10px;
      background: #13B5B7;
      color: #0D1014;
      font-weight: bold;
      font-size: 12px;
      border-radius: 3px;
      cursor: pointer;
      text-decoration: none;
      position: relative;
    `

    const tooltip = document.createElement('span')
    tooltip.style.cssText = `
      display: none;
      position: absolute;
      left: 0;
      top: calc(100% + 6px);
      z-index: 2147483647;
      width: max-content;
      max-width: 360px;
      padding: 6px 8px;
      background: #0D1014;
      color: #13B5B7;
      border: 1px solid #13B5B7;
      border-radius: 3px;
      font: 12px monospace;
      white-space: normal;
      pointer-events: none;
    `

    const setLog = (message) => {
      calderaBtn.title = message
      calderaBtn.dataset.calderaLog = message
      tooltip.textContent = message
    }

    setLog('CALDERA ready')
    calderaBtn.appendChild(tooltip)
    calderaBtn.addEventListener('mouseenter', () => { tooltip.style.display = 'block' })
    calderaBtn.addEventListener('mouseleave', () => { tooltip.style.display = 'none' })

    calderaBtn.addEventListener('click', (e) => {
      e.preventDefault()
      setLog('Sending to CALDERA...')

      const pathParts = window.location.pathname.split('/')
      const gameIdx = pathParts.indexOf('games')
      const modIdx = pathParts.indexOf('mods')
      const gameDomain = gameIdx !== -1 ? pathParts[gameIdx + 1] : pathParts[1] || null
      const modId = modIdx !== -1 ? pathParts[modIdx + 1] : null

      const parsedFileUrl = new URL(fileUrl)
      const fileId = parsedFileUrl.searchParams.get('file_id') || parsedFileUrl.searchParams.get('id') || null

      console.log('CALDERA sending download request', { fileUrl, gameDomain, modId, fileId })

      browser.runtime.sendMessage({
        type: 'CALDERA_DOWNLOAD',
        url: btn.href,
        game_domain: gameDomain,
        mod_id: modId,
        file_id: fileId
      }).then((response) => {
        if (response?.success) {
          setLog('CALDERA queued download')
        } else {
          const error = response?.error || 'download request failed'
          setLog(error)
        }
      }).catch((err) => {
        console.error('CALDERA message error:', err)
        setLog(err.message)
      })
    })

    btn.insertAdjacentElement('afterend', calderaBtn)
  })
}

injectButtons()

const observer = new MutationObserver(injectButtons)
observer.observe(document.body, { childList: true, subtree: true })
