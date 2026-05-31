<script>
  import { push } from 'svelte-spa-router';

  export let settingsActive = false;
  export let backRoute = '';
</script>

<div class="topbar">
  <div class="topbar-inner">
    <div class="left-cluster">
      {#if backRoute}
        <button class="back-btn" on:click={() => push(backRoute)}>&lt; BACK</button>
      {/if}
      <button class="logo-btn" on:click={() => push(backRoute || '/')}>
        <img class="logo" src="/caldera_logo.svg" alt="CALDERA" />
      </button>
      <button class="library-tab" on:click={() => push('/')}>&gt;&gt; GAME LIBRARY</button>
    </div>

    <button
      class={`settings-btn ${settingsActive ? 'active' : ''}`}
      on:click={() => push('/settings')}
      aria-label="Settings"
    >⚙</button>
  </div>
</div>

<style>
  .topbar {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    border-bottom: var(--border-subtle);
    background: var(--bg-surface);
    z-index: 20;
    padding: 10px 32px;
  }

  .topbar-inner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
  }

  .left-cluster {
    display: inline-flex;
    width: fit-content;
    align-items: center;
    gap: 12px;
  }

  .logo-btn {
    border: 0;
    background: transparent;
    padding: 0;
    cursor: pointer;
  }

  .logo {
    height: 32px;
    width: auto;
    color: var(--text);
  }

  .library-tab {
    border: var(--border);
    background: transparent;
    color: var(--interactive);
    font-family: var(--font);
    height: fit-content;
    padding: 6px 10px;
    text-transform: uppercase;
    border-radius: var(--border-radius);
  }

  .back-btn {
    border: var(--border-subtle);
    background: transparent;
    color: var(--text);
    font-family: var(--font);
    padding: 6px 10px;
    text-transform: uppercase;
    border-radius: var(--border-radius);
    cursor: pointer;
  }

  .back-btn:hover {
    border: var(--border);
    color: var(--interactive);
  }

  .library-tab:hover {
    color: var(--action);
  }

  .settings-btn {
    margin-left: auto;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-family: var(--font);
    padding: 6px 10px;
    cursor: pointer;
    border-radius: var(--border-radius);
    box-shadow: none;
  }

  .settings-btn:hover,
  .settings-btn.active {
    border: var(--border-subtle);
    color: var(--interactive);
  }
</style>
