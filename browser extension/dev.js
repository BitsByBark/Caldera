const banner = document.createElement('div')
banner.style.cssText = `
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  z-index: 2147483647;
  background: #0D1014;
  border-bottom: 3px solid #13B5B7;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 10px;
  font-family: monospace;
  font-size: 14px;
  font-weight: bold;
  color: #13B5B7;
`

const img = document.createElement('img')
img.src = 'https://i.imgflip.com/26am.jpg'
img.style.cssText = 'height: 60px; border-radius: 4px;'

const text = document.createElement('span')
text.textContent = 'CALDERA EXTENSION LOADED'

banner.appendChild(img)
banner.appendChild(text)
document.documentElement.prepend(banner)

document.body.style.marginTop = `${banner.offsetHeight}px`
