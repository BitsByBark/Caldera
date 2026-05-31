<script>
  import { open } from '@tauri-apps/plugin-dialog';

  export let value = '';
  export let onChange = () => {};

  async function chooseDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (typeof selected === 'string' && selected.length > 0) {
      onChange(selected);
    }
  }
</script>

<div class="input-shell">
  <input class="input-text" value={value} on:input={(e) => onChange(e.target.value)} />
  <button type="button" class="dots" on:click={chooseDirectory}>...</button>
</div>

<style>
  .input-shell { border: var(--border-subtle); background: var(--bg-surface); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; border-radius: var(--border-radius); }
  .input-shell:focus-within { border: var(--border); }
  .input-text { width: 100%; border: 0; outline: 0; font-family: var(--font); color: var(--text); background: transparent; }
  .dots { border: 0; border-left: var(--border-subtle); padding: 0 8px; background: transparent; color: var(--text-muted); border-radius: var(--border-radius); }
</style>
