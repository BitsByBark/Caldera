<script>
  import BaseListItem from './BaseListItem.svelte';
  import Toggle from './Toggle.svelte';
  import TextInput from './TextInput.svelte';
  import PathInput from './PathInput.svelte';
  import Dropdown from './Dropdown.svelte';
  import Cycle from './Cycle.svelte';
  import RangeInput from './RangeInput.svelte';
  import ColorInput from './ColorInput.svelte';
  import KeybindInput from './KeybindInput.svelte';

  export let entry;
  export let value = entry.default;
  export let onChange = () => {};
</script>

<BaseListItem description={entry.desc || ''}>
  <span slot="left">"{entry.label}"</span>
  <div slot="right">
    {#if entry.type.kind === 'bool'}
      <Toggle value={value} onChange={onChange} />
    {:else if entry.type.kind === 'text'}
      <TextInput value={value} onChange={onChange} />
    {:else if entry.type.kind === 'path'}
      <PathInput value={value} onChange={onChange} />
    {:else if entry.type.kind === 'select'}
      <Dropdown options={['none', 'skyrim', 'fallout4']} value={value} onChange={onChange} />
    {:else if entry.type.kind === 'cycle'}
      <Cycle options={entry.type.options} value={value} onChange={onChange} />
    {:else if entry.type.kind === 'range'}
      <RangeInput min={entry.type.min} max={entry.type.max} step={entry.type.step} value={value} onChange={onChange} />
    {:else if entry.type.kind === 'color'}
      <ColorInput value={value} onChange={onChange} />
    {:else if entry.type.kind === 'keybind'}
      <KeybindInput value={value} onChange={onChange} />
    {/if}
  </div>
</BaseListItem>
