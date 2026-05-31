<script>
  import BaseListItem from './BaseListItem.svelte';
  import Button from './Button.svelte';
  export let mod;
</script>

<BaseListItem expandable={true}>
  <span slot="left">"{mod.name}"</span>
  <div slot="right" class="btns">
    <Button variant="primary" icon="+" label="DEPLOY" />
    <Button variant="destructive" icon="-" label="DELETE" />
  </div>
  <div class="kv">
    <div>description:</div><div>{mod.description}</div>
    <div>version:</div><div>{mod.version}</div>
    <div>size:</div><div>{mod.size}</div>
    <div>source:</div><div>{mod.source}</div>
  </div>
</BaseListItem>

<style>
  .btns :global(button) { width: auto; }
  .kv { display: grid; grid-template-columns: auto 1fr; column-gap: 8px; row-gap: 4px; color: var(--text-muted); }
</style>
