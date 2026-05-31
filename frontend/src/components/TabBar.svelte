<script>
  export let tabs = [];
  export let active = '';
  export let orientation = 'horizontal';
  export let onChange = () => {};
</script>

<div class={orientation === 'vertical' ? 'tabs-v' : 'tabs-h'}>
  {#each tabs as tab}
    <button class={`tab ${active === tab.id ? 'active' : ''}`} on:click={() => onChange(tab.id)}>
      {#if active === tab.id}>> "{tab.label}"{:else}"{tab.label}"{/if}
    </button>
  {/each}
</div>

<style>
  .tabs-h, .tabs-v { display: flex; gap: 6px; }
  .tabs-v { flex-direction: column; }
  .tab { border: 1px solid transparent; background: transparent; color: var(--text-muted); font-family: var(--font); padding: 4px 6px; text-align: left; border-radius: var(--border-radius); }
  .tab:hover { color: var(--action); }
  .tab.active { border: var(--border); color: var(--interactive); }
</style>
