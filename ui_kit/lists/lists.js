export function initLists(root = document) {
  root.querySelectorAll('[data-expandable]').forEach((item) => {
    const trigger = item.querySelector('[data-expand-trigger]');
    const icon = item.querySelector('[data-expand-icon]');
    const hasDetails = !!item.querySelector('[data-expand-content]');
    if (!trigger || !hasDetails) return;
    trigger.addEventListener('click', () => {
      const expanded = item.classList.toggle('expanded');
      if (icon) icon.textContent = expanded ? '^' : 'v';
    });
  });
}
