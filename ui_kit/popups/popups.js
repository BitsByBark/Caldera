let overlay;
let titleNode;
let bodyNode;
let cancelBtn;
let confirmBtn;
let toastStack;

function ensureModal() {
  if (overlay) return;
  overlay = document.createElement('div');
  overlay.className = 'modal-overlay';
  overlay.innerHTML = `
    <div class="modal-box" data-modal-box>
      <div data-modal-title>{ TITLE }</div>
      <div data-modal-body></div>
      <div class="modal-actions">
        <button class="btn btn-primary btn-inline" data-modal-confirm>CONFIRM</button>
        <button class="btn btn-secondary btn-inline" data-modal-cancel>CANCEL</button>
      </div>
    </div>
  `;
  document.body.appendChild(overlay);
  titleNode = overlay.querySelector('[data-modal-title]');
  bodyNode = overlay.querySelector('[data-modal-body]');
  cancelBtn = overlay.querySelector('[data-modal-cancel]');
  confirmBtn = overlay.querySelector('[data-modal-confirm]');

  overlay.addEventListener('click', (event) => {
    if (event.target === overlay) closeModal();
  });
  cancelBtn.addEventListener('click', closeModal);
  confirmBtn.addEventListener('click', closeModal);
}

function closeModal() {
  overlay?.classList.remove('open');
}

function ensureToastStack() {
  if (toastStack) return;
  toastStack = document.createElement('div');
  toastStack.className = 'toast-stack';
  document.body.appendChild(toastStack);
}

export function openModal(title, body) {
  ensureModal();
  titleNode.textContent = `{ ${String(title).toUpperCase()} }`;
  bodyNode.textContent = body;
  overlay.classList.add('open');
}

export function showToast(message, type = 'info') {
  ensureToastStack();
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.textContent = message;
  toastStack.appendChild(toast);

  setTimeout(() => {
    toast.classList.add('fade');
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}
