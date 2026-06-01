<script>
  export let expandable = false;
  export let description = '';
  let expanded = false;
  $: canExpand = expandable || !!description;
  const toggle = () => { if (canExpand) expanded = !expanded; };
</script>

<div class={`list-item ${expanded ? 'expanded' : ''}`}>
  <div class="list-main" on:click={toggle}>
    <div class="list-left"><slot name="left" /></div>
    <div class="list-right"><slot name="right" /></div>
    <div class="list-expand">{#if canExpand}{expanded ? '↑' : '↓'}{/if}</div>
  </div>
  {#if canExpand && expanded}
    <div class="list-details">
      {#if description}<div class="desc">{description}</div>{/if}
      <slot />
    </div>
  {/if}
</div>

<style>
  .list-item { width: 100%; border-bottom: var(--border-subtle); }
  .list-main { min-height: 34px; display: grid; grid-template-columns: 1fr auto auto; gap: 8px; align-items: center; padding: 6px 8px; border-radius: var(--border-radius); }
  .list-main:hover { background: var(--bg-hover); }
  .list-right { display: flex; gap: 8px; align-items: center; }
  .list-expand { min-width: 14px; text-align: right; color: var(--interactive); }
  .list-details { padding: 8px; }
  .desc { color: var(--text-muted); }
</style>
