<script>
  export let options = [];
  export let value = '';
  export let onChange = () => {};
  const shift = (dir) => {
    if (!options.length) return;
    const i = options.indexOf(value);
    const next = (i + dir + options.length) % options.length;
    onChange(options[next]);
  };
</script>

<div class="input-shell cycle">
  <button on:click={() => shift(-1)}>&lt;</button>
  <span>{value}</span>
  <button on:click={() => shift(1)}>&gt;</button>
</div>

<style>
  .input-shell { border: var(--border-subtle); background: var(--bg-surface); color: var(--text); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; border-radius: var(--border-radius); }
  .cycle { justify-content: space-between; }
  button { border: 0; background: transparent; color: var(--interactive); cursor: pointer; width: 20px; border-radius: var(--border-radius); }
  button:hover { color: var(--action); }
</style>
