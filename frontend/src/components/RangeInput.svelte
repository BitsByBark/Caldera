<script>
  export let min = 0;
  export let max = 100;
  export let step = 1;
  export let value = 0;
  export let onChange = () => {};
</script>

<div class="input-shell range-wrap">
  <input type="range" {min} {max} {step} {value} on:input={(e) => onChange(Number(e.target.value))} />
  <span>{value}</span>
</div>

<style>
  .input-shell { border: var(--border-subtle); background: var(--bg-surface); color: var(--text); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; gap: 8px; border-radius: var(--border-radius); }
  input { width: 100%; accent-color: var(--slider-track); }
  span { min-width: 44px; text-align: right; color: var(--text); }
</style>
