function keyCombo(event) {
  const parts = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.shiftKey) parts.push('Shift');
  if (event.altKey) parts.push('Alt');
  if (event.metaKey) parts.push('Meta');
  const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  if (!['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) parts.push(key);
  return parts.join('+') || '...';
}

export function initInputs(root = document) {
  root.querySelectorAll('[data-dropdown]').forEach((dropdown) => {
    const button = dropdown.querySelector('[data-dropdown-button]');
    const label = dropdown.querySelector('[data-dropdown-label]');
    const items = dropdown.querySelectorAll('[data-dropdown-item]');
    button?.addEventListener('click', () => {
      dropdown.classList.toggle('open');
    });
    items.forEach((item) => {
      item.addEventListener('click', () => {
        label.textContent = `"${item.textContent.trim()}"`;
        dropdown.classList.remove('open');
      });
    });
  });

  root.querySelectorAll('[data-cycle]').forEach((node) => {
    const options = (node.dataset.options || '').split(',').map((s) => s.trim()).filter(Boolean);
    const label = node.querySelector('[data-cycle-label]');
    const prev = node.querySelector('[data-cycle-prev]');
    const next = node.querySelector('[data-cycle-next]');
    let idx = Math.max(0, options.indexOf(node.dataset.value || options[0]));
    const paint = () => { label.textContent = options[idx] || ''; };
    prev?.addEventListener('click', () => { idx = (idx - 1 + options.length) % options.length; paint(); });
    next?.addEventListener('click', () => { idx = (idx + 1) % options.length; paint(); });
    paint();
  });

  root.querySelectorAll('[data-range]').forEach((wrap) => {
    const input = wrap.querySelector('input[type="range"]');
    const value = wrap.querySelector('[data-range-value]');
    const sync = () => { value.textContent = input.value; };
    input?.addEventListener('input', sync);
    sync();
  });

  root.querySelectorAll('[data-toggle]').forEach((toggle) => {
    toggle.addEventListener('click', () => {
      toggle.classList.toggle('on');
    });
  });

  root.querySelectorAll('[data-color-input]').forEach((node) => {
    const hidden = node.querySelector('input[type="color"]');
    const swatch = node.querySelector('[data-color-swatch]');
    const text = node.querySelector('[data-color-value]');
    node.addEventListener('click', () => hidden?.click());
    hidden?.addEventListener('input', () => {
      swatch.style.background = hidden.value;
      text.textContent = hidden.value.toUpperCase();
    });
  });

  root.querySelectorAll('[data-keybind]').forEach((node) => {
    const value = node.querySelector('[data-keybind-value]');
    let capture = false;
    node.addEventListener('click', () => {
      capture = true;
      node.classList.add('capturing');
      value.textContent = '...';
    });
    node.addEventListener('keydown', (event) => {
      if (!capture) return;
      event.preventDefault();
      if (event.key === 'Escape') {
        capture = false;
        node.classList.remove('capturing');
        return;
      }
      value.textContent = keyCombo(event);
      capture = false;
      node.classList.remove('capturing');
    });
  });
}
