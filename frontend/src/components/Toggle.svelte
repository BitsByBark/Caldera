<script>
  export let value = false;
  export let onChange = () => {};
  const toggle = () => onChange(!value);
</script>

<div class={`toggle ${value ? 'on' : ''}`} on:click={toggle}>
  <div class="toggle-pill"></div>
</div>

<style>
  .toggle { display: inline-flex; align-items: center; cursor: pointer; }
  .toggle-pill { width: 36px; height: 20px; border: 0; background: var(--toggle-off); position: relative; border-radius: var(--border-radius); transition: 150ms ease; }
  .toggle-pill::after { content: ''; width: 14px; height: 14px; position: absolute; top: 3px; left: 2px; background: var(--text-muted); border-radius: var(--border-radius); transition: 150ms ease; }
  .toggle.on .toggle-pill { background: var(--toggle-on); }
  .toggle.on .toggle-pill::after { left: 20px; background: var(--text); }
</style>
