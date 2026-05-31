<script>
  import { push } from 'svelte-spa-router';

  export let settingsActive = false;
</script>

<div class="topbar">
  <div class="topbar-inner">
    <div class="left-cluster">
      <img class="logo" src="/caldera_logo.svg" alt="CALDERA" />
      <button class="library-tab">&gt;&gt; GAME LIBRARY</button>
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
    width: fit-content; height: fit-content;
   align-items: bottom;
    gap: 12px;
  }

  .logo {
    height: 75;
    width: auto;
    color: var(--text);
  }

  .library-tab {
    border: var(--border);
    background: transparent;
    color: var(--interactive);
    font-family: var(--font);
    padding: 6px 10px;
    text-transform: uppercase;
    border-radius: var(--border-radius);
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
