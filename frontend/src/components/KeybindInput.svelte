<script>
  export let value = 'Ctrl+Shift+T';
  export let onChange = () => {};
  let capture = false;
  const combo = (e) => {
    const p = [];
    if (e.ctrlKey) p.push('Ctrl');
    if (e.shiftKey) p.push('Shift');
    if (e.altKey) p.push('Alt');
    if (e.metaKey) p.push('Meta');
    if (!['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) p.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);
    return p.join('+') || '...';
  };
</script>

<div class={`input-shell ${capture ? 'cap' : ''}`} tabindex="0" on:click={() => (capture = true)} on:keydown={(e) => {
  if (!capture) return;
  e.preventDefault();
  if (e.key === 'Escape') { capture = false; return; }
  onChange(combo(e));
  capture = false;
}}>{capture ? '...' : value}</div>

<style>
  .input-shell { border: var(--border-subtle); background: var(--bg-surface); color: var(--text); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; cursor: pointer; border-radius: var(--border-radius); }
  .cap { border: var(--border); color: var(--text-muted); }
</style>
