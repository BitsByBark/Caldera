<script>
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { get } from 'svelte/store';
  import TopBar from '../components/TopBar.svelte';
  import Button from '../components/Button.svelte';
  import Cycle from '../components/Cycle.svelte';
  import { gameList } from '../stores/game.js';
  import { settings } from '../stores/settings.js';
  import { sessionLog, addLog } from '../stores/log.js';
  import { showToast } from '../components/Toast.svelte';

  export let params = {};

  const deployerOptions = ['CYBERPUNK 2077', 'UNREAL ENGINE', 'NONE'];

  let game = null;
  let gameConfig = {
    game_id: '',
    name: '',
    mod_directory: '',
    deployer: null,
    active_profile: null,
    profiles: [],
  };
  let artwork = { hero: '', logo: '' };
  let editMode = false;
  let logExpanded = false;
  let logViewport;

  $: game = $gameList.find((g) => g.app_id === params.id) || null;
  $: profileOptions = gameConfig.profiles?.length ? gameConfig.profiles : ['^ CREATE PROFILE'];
  $: activeProfileValue = gameConfig.profiles?.length ? (gameConfig.active_profile || gameConfig.profiles[0]) : '^ CREATE PROFILE';
  $: deployerValue = gameConfig.deployer || '^ SELECT DEPLOYER';
  $: lastLog = $sessionLog.length ? $sessionLog[$sessionLog.length - 1] : { time: '--:--', message: 'No log entries yet', type: 'default' };

  $: if (logExpanded) {
    scrollLogToBottom();
  }

  onMount(async () => {
    const steamPath = get(settings).steam_path || null;

    try {
      gameConfig = await invoke('get_game_config', { gameId: params.id });
    } catch (err) {
      showToast(String(err), 'error');
    }

    try {
      artwork = await invoke('get_game_artwork', { appId: params.id, steamPath });
    } catch (err) {
      showToast(String(err), 'error');
    }

    try {
      await invoke('ensure_game_cache', { appId: params.id, steamPath });
    } catch (err) {
      showToast(String(err), 'error');
    }

    addLog('"CALDERA" initialised successfully', 'success');
    addLog(`Loading game: "${game?.name || gameConfig.name || params.id}"`, 'info');
    addLog('No deployer configured — select one to get started', 'warning');
  });

  function onSetup() {
    showToast('Setup wizard coming soon', 'info');
    addLog('Setup wizard requested', 'info');
  }

  function toggleEditMode() {
    editMode = !editMode;
    addLog(editMode ? 'Manager mode toggle requested' : 'Manager mode toggle cleared', 'info');
  }

  function onProfileChange(v) {
    if (!gameConfig.profiles?.length || v === '^ CREATE PROFILE') {
      showToast('Profile creation coming soon', 'info');
      addLog('Profile creation requested', 'info');
      return;
    }

    gameConfig = { ...gameConfig, active_profile: v };
    addLog(`Active profile set to ${v}`, 'success');
  }

  function onDeployerChange(v) {
    const next = v === 'NONE' ? null : v;
    gameConfig = { ...gameConfig, deployer: next };
    addLog(`Deployer set to ${v}`, 'success');
  }

  async function scrollLogToBottom() {
    await tick();
    if (logViewport) {
      logViewport.scrollTop = logViewport.scrollHeight;
    }
  }
</script>

<section class="page">
  <TopBar backRoute="/" />

  <main class="content">
    <section class="hero">
      {#if artwork.hero}
        <img class="hero-image" src={artwork.hero} alt={game?.name || gameConfig.name || params.id} />
      {:else}
        <div class="hero-fallback"></div>
      {/if}

      <div class="hero-logo">
        {#if artwork.logo}
          <img src={artwork.logo} alt={game?.name || gameConfig.name || params.id} />
        {:else}
          <div class="hero-name">{game?.name || gameConfig.name || params.id}</div>
        {/if}
      </div>

      <div class="hero-setup">
        <Button variant="secondary" label="// SETUP" onClick={onSetup} />
      </div>
    </section>

    <section class="action-bar">
      <button class={`edit-btn ${editMode ? 'active' : ''}`} on:click={toggleEditMode}>
        {editMode ? '// DONE' : '// EDIT'}
      </button>

      <div class="divider"></div>

      <div class="selector">
        <span class="selector-label">PROFILE :</span>
        <div class="selector-input">
          <Cycle options={profileOptions} value={activeProfileValue} onChange={onProfileChange} />
        </div>
      </div>

      <div class="divider"></div>

      <div class="selector">
        <span class="selector-label">DEPLOYER =</span>
        <div class="selector-input">
          <Cycle options={deployerOptions} value={deployerValue} onChange={onDeployerChange} />
        </div>
      </div>
    </section>
  </main>

  <section class={`log-panel ${logExpanded ? 'expanded' : 'collapsed'}`}>
    <div class="log-head">
      <div class="preview"><span class="time">{lastLog.time}</span><span class="pipe"> | </span>{lastLog.message}</div>
      <button class="toggle" on:click={() => (logExpanded = !logExpanded)}>{logExpanded ? '^' : 'v'}</button>
    </div>

    {#if logExpanded}
      <div class="log-body" bind:this={logViewport}>
        {#each $sessionLog as entry}
          <div class={`line ${entry.type || 'default'}`}>
            <span class="time">{entry.time}</span><span class="pipe"> | </span><span>{entry.message}</span>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</section>

<style>
  .page {
    min-height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font);
    display: flex;
    flex-direction: column;
  }

  .content {
    padding-top: 56px;
    padding-bottom: 36px;
  }

  .hero {
    position: relative;
    height: 340px;
    width: 100%;
    border-bottom: var(--border-subtle);
    overflow: hidden;
    background: var(--bg-surface);
  }

  .hero-image {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center top;
    display: block;
  }

  .hero-fallback {
    width: 100%;
    height: 100%;
    background: var(--bg-surface);
  }

  .hero-logo {
    position: absolute;
    left: 24px;
    bottom: 24px;
    z-index: 2;
  }

  .hero-logo img {
    max-height: 80px;
    max-width: 300px;
    object-fit: contain;
    display: block;
  }

  .hero-name {
    font-size: 28px;
    font-weight: 700;
    color: var(--text);
  }

  .hero-setup {
    position: absolute;
    right: 24px;
    bottom: 24px;
    width: 160px;
    z-index: 2;
  }

  .action-bar {
    width: 100%;
    background: var(--bg-surface);
    border-bottom: var(--border-subtle);
    padding: 12px 24px;
    display: flex;
    align-items: center;
    gap: 24px;
    box-sizing: border-box;
  }

  .edit-btn {
    border: var(--border-subtle);
    background: transparent;
    color: var(--text);
    font-family: var(--font);
    padding: 8px 12px;
    border-radius: var(--border-radius);
    cursor: pointer;
  }

  .edit-btn.active {
    border: var(--border-action);
    color: var(--action);
  }

  .edit-btn:hover {
    border: var(--border);
    color: var(--interactive);
  }

  .divider {
    width: 1px;
    height: 20px;
    background: var(--ash);
  }

  .selector {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .selector-label {
    color: var(--text-muted);
  }

  .selector-input {
    width: 240px;
  }

  .log-panel {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--bg-surface);
    border-top: var(--border-subtle);
    z-index: 25;
  }

  .log-panel.expanded {
    height: 180px;
    border: var(--border-subtle);
    background: var(--bg);
  }

  .log-head {
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    border-bottom: 0;
    box-sizing: border-box;
  }

  .log-panel.expanded .log-head {
    border-bottom: var(--border-subtle);
  }

  .preview {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .toggle {
    border: 0;
    background: transparent;
    color: var(--interactive);
    font-family: var(--font);
    cursor: pointer;
    font-size: 16px;
    padding: 0 8px;
  }

  .log-body {
    height: calc(180px - 36px);
    overflow-y: auto;
    padding: 8px 12px;
    box-sizing: border-box;
  }

  .line {
    padding: 2px 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .time {
    color: var(--text-muted);
  }

  .pipe {
    color: var(--ash);
  }

  .line.default { color: var(--text); }
  .line.success { color: var(--success); }
  .line.error { color: var(--action); }
  .line.warning { color: var(--warning); }
  .line.info { color: var(--interactive); }
</style>
