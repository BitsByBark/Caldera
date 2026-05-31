<script>
  export let value = '#E63636';
  export let onChange = () => {};
  let picker;
</script>

<div class="input-shell" on:click={() => picker.click()}>
  <div class="swatch" style={`background:${value}`}></div>
  <div>{value.toUpperCase()}</div>
  <input bind:this={picker} type="color" value={value} hidden on:input={(e) => onChange(e.target.value.toUpperCase())} />
</div>

<style>
  .input-shell { border: var(--border-subtle); background: var(--bg-surface); color: var(--text); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; gap: 8px; cursor: pointer; border-radius: var(--border-radius); }
  .input-shell:hover, .input-shell:focus-within { border: var(--border); }
  .swatch { width: 18px; height: 18px; border: var(--border-subtle); border-radius: var(--border-radius); }
</style>
