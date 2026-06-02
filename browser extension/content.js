function injectButtons() {
  const manualButtons = document.querySelectorAll(
    'a[class*="btn-manual"], a.nxm-button[href*="file_id="]'
  )

  manualButtons.forEach((btn) => {
    if (btn.nextSibling?.classList?.contains('caldera-dl-btn')) return

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
    `

    calderaBtn.addEventListener('click', (e) => {
      e.preventDefault()

      const pathParts = window.location.pathname.split('/')
      const gameIdx = pathParts.indexOf('games')
      const modIdx = pathParts.indexOf('mods')
      const gameDomain = gameIdx !== -1 ? pathParts[gameIdx + 1] : pathParts[1] || null
      const modId = modIdx !== -1 ? pathParts[modIdx + 1] : null

      const parsedFileUrl = new URL(fileUrl)
      const fileId = parsedFileUrl.searchParams.get('file_id') || parsedFileUrl.searchParams.get('id') || null

      chrome.runtime.sendMessage({
        type: 'CALDERA_DOWNLOAD',
        url: btn.href,
        game_domain: gameDomain,
        mod_id: modId,
        file_id: fileId
      })

      calderaBtn.textContent = '✓ QUEUED'
      calderaBtn.style.background = '#84E052'
    })

    btn.insertAdjacentElement('afterend', calderaBtn)
  })
}

injectButtons()

const observer = new MutationObserver(injectButtons)
observer.observe(document.body, { childList: true, subtree: true })
