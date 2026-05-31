<script>
  export let variant = 'secondary';
  export let icon = '';
  export let label = '';
  export let onClick = () => {};
</script>

<button class={`btn btn-${variant}`} on:click={onClick}>
  {#if icon}{icon} {/if}{label}
</button>

<style>
  .btn {
    font-family: var(--font);
    text-transform: uppercase;
    border-radius: var(--border-radius);
    border: var(--border-subtle);
    padding: 8px 10px;
    cursor: pointer;
    display: block;
    width: 100%;
    background: transparent;
    color: var(--text);
  }

  .btn:hover { opacity: 0.9; }
  .btn:active { opacity: 0.8; }

  .btn-secondary {
    background: transparent;
    color: var(--text);
    border: var(--border-subtle);
  }

  .btn-secondary:hover {
    border: var(--border);
    color: var(--interactive);
  }

  .btn-primary {
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
    border: 1px solid transparent;
  }

  .btn-destructive {
    background: var(--btn-destructive-bg);
    color: var(--btn-destructive-text);
    border: 1px solid transparent;
  }
</style>
