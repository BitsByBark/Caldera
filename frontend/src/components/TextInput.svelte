<script>
  export let value = '';
  export let placeholder = '"display name"';
  export let onChange = () => {};
</script>

<div class="input-shell">
  <input class="input-text" {value} {placeholder} on:input={(e) => onChange(e.target.value)} />
</div>

<style>
  .input-shell { border: var(--border-subtle); background: var(--bg-surface); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; border-radius: var(--border-radius); }
  .input-shell:focus-within { border: var(--border); }
  .input-text { width: 100%; border: 0; outline: 0; font-family: var(--font); color: var(--text); background: transparent; }
  .input-text::placeholder { color: var(--text-muted); }
</style>
