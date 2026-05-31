<script>
  export let value = '';
  export let onChange = () => {};
</script>

<div class="input-shell">
  <input class="input-text" value={value} on:input={(e) => onChange(e.target.value)} />
  <button type="button" class="dots">...</button>
</div>

<style>
  .input-shell { border: var(--border-subtle); background: var(--bg-surface); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; border-radius: var(--border-radius); }
  .input-shell:focus-within { border: var(--border); }
  .input-text { width: 100%; border: 0; outline: 0; font-family: var(--font); color: var(--text); background: transparent; }
  .dots { border: 0; border-left: var(--border-subtle); padding: 0 8px; background: transparent; color: var(--text-muted); border-radius: var(--border-radius); }
</style>
