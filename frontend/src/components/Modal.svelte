<script>
  export let open = false;
  export let title = 'TITLE';
  export let body = '';
  export let onConfirm = () => {};
  export let onCancel = () => {};
  $: heading = `{ ${String(title).toUpperCase()} }`;
</script>

{#if open}
  <div class="overlay" on:click={(e) => e.target === e.currentTarget && onCancel()}>
    <div class="box">
      <div>{heading}</div>
      <div>{body}</div>
      <div class="actions">
        <button class="btn primary" on:click={onConfirm}>CONFIRM</button>
        <button class="btn secondary" on:click={onCancel}>CANCEL</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay { position: fixed; inset: 0; background: rgba(26,22,20,0.85); display: flex; align-items: center; justify-content: center; padding: 16px; }
  .box { width: min(560px, 100%); border: var(--border-subtle); background: var(--bg-surface); padding: 12px; display: flex; flex-direction: column; gap: 12px; border-radius: var(--border-radius); color: var(--text-muted); }
  .box > :first-child { color: var(--text); }
  .actions { display: flex; gap: 8px; }
  .btn { font-family: var(--font); text-transform: uppercase; border-radius: var(--border-radius); border: var(--border-subtle); padding: 8px 10px; cursor: pointer; }
  .secondary { background: transparent; color: var(--text); }
  .secondary:hover { border: var(--border); color: var(--interactive); }
  .primary { background: var(--btn-primary-bg); color: var(--btn-primary-text); border: 1px solid transparent; }
</style>
