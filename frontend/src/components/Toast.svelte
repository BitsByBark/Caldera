<script context="module">
  import { writable } from 'svelte/store';

  export const toastStore = writable([]);
  let seq = 0;

  export function showToast(message, type = 'info') {
    const id = ++seq;
    toastStore.update((list) => [...list, { id, message, type }]);
    setTimeout(() => {
      toastStore.update((list) => list.filter((t) => t.id !== id));
    }, 3000);
  }
</script>

<script>
</script>

<div class="stack">
  {#each $toastStore as toast (toast.id)}
    <div class={`toast ${toast.type}`}>{toast.message}</div>
  {/each}
</div>

<style>
  .stack { position: fixed; right: 12px; bottom: 12px; display: flex; flex-direction: column; gap: 8px; }
  .toast { background: var(--bg-surface); border: var(--border-subtle); border-radius: var(--border-radius); color: var(--text); padding: 8px 10px; min-width: 220px; }
  .toast.info { border: var(--border); }
  .toast.success { border: 1px solid var(--success); }
  .toast.warning { border: 1px solid var(--warning); }
  .toast.error { border: var(--border-action); }
</style>
