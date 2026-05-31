<script>
  import BaseListItem from './BaseListItem.svelte';
  import Toggle from './Toggle.svelte';
  export let profile;
</script>

<BaseListItem expandable={true}>
  <span slot="left">"{profile.name}"</span>
  <div slot="right">
    <Toggle value={profile.enabled} onChange={(v) => (profile.enabled = v)} />
  </div>
  <div class="mods">
    {#each profile.mods as mod}
      <div class="mod-row">
        <span>"{mod.name}"</span>
        <Toggle value={mod.enabled} onChange={(v) => (mod.enabled = v)} />
      </div>
    {/each}
  </div>
</BaseListItem>

<style>
  .mods { display: flex; flex-direction: column; gap: 6px; }
  .mod-row {
    border-bottom: var(--border-subtle);
    background: transparent;
    padding: 6px 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-radius: var(--border-radius);
  }
  .mod-row:hover { background: var(--bg-hover); }
</style>
