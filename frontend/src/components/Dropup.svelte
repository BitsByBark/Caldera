<script>
  export let options = [];
  export let value = '';
  export let onChange = () => {};
  let open = false;
</script>

<div class="dropdown">
  <button class="input-shell head" on:click={() => (open = !open)}><span class="arr">↑</span><span>{value}</span><span class="arr">↑</span></button>
  {#if open}
    <div class="list">
      {#each options as opt}
        <div class="item" on:click={() => { onChange(opt); open = false; }}>{opt}</div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dropdown { position: relative; }
  .input-shell { width: 100%; border: var(--border-subtle); background: var(--bg-surface); color: var(--text); min-height: 30px; display: flex; align-items: center; padding: 4px 8px; border-radius: var(--border-radius); font-family: var(--font-ui); }
  .head { justify-content: space-between; cursor: pointer; }
  .arr { color: var(--interactive); }
  .dropdown:focus-within .input-shell, .list { border-color: var(--interactive); }
  .list { position: absolute; bottom: 100%; top: auto; left: 0; right: 0; margin-bottom: 2px; margin-top: 0; border: var(--border-subtle); background: var(--bg-surface); border-top: 0; border-radius: var(--border-radius); }
  .item { padding: 6px 8px; cursor: pointer; color: var(--text); font-family: var(--font-ui); }
  .item:hover { background: var(--bg-hover); color: var(--action); }
</style>
