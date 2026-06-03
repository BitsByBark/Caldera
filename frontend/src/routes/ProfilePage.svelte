<script>
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { get } from 'svelte/store';
  import TopBar from '../components/TopBar.svelte';
  import Dropdown from '../components/Dropdown.svelte';
  import { gameList } from '../stores/game.js';
  import { settings } from '../stores/settings.js';
  import { sessionLog, addLog } from '../stores/log.js';

  export let params = {};

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
  let logExpanded = false;
  let logViewport;
  let activePaneTab = 'DEFAULT_PROFILE';
  let deployerCatalog = [];
  let profileModRows = [];
  let modlistRows = [];
  const leftTabs = [
    { id: 'DOWNLOADED_MODS', label: 'DOWNLOADED MODS' },
    { id: 'COLLECTIONS', label: 'COLLECTIONS' },
  ];
  const defaultProfileTab = { id: 'DEFAULT_PROFILE', label: 'DEFAULT PROFILE' };

  $: game = $gameList.find((g) => g.app_id === params.id) || null;
  $: profileTabs = (gameConfig.profiles || []).length ? gameConfig.profiles : ['DEFAULT'];
  $: activeProfileTab = (gameConfig.active_profile && profileTabs.includes(gameConfig.active_profile))
    ? gameConfig.active_profile
    : profileTabs[0];
  $: profileOptions = (gameConfig.profiles || []).length
    ? [...gameConfig.profiles, '^ CREATE PROFILE']
    : ['DEFAULT', '^ CREATE PROFILE'];
  $: activeProfileValue = (gameConfig.profiles || []).length
    ? activeProfileTab
    : 'DEFAULT';
  $: deployerLabelById = new Map(deployerCatalog.map((d) => [d.id, d.name.toUpperCase()]));
  $: deployerIdByLabel = new Map(deployerCatalog.map((d) => [d.name.toUpperCase(), d.id]));
  $: deployerOptions = [...deployerCatalog.map((d) => d.name.toUpperCase()), 'NONE'];
  $: deployerValue = !gameConfig.deployer || gameConfig.deployer === 'NONE' ? 'SELECT DEPLOYER' : deployerLabelById.get(gameConfig.deployer) || gameConfig.deployer;

  $: if (logExpanded) scrollLogToBottom();

  function toAssetUrl(path) {
    if (!path) return null;
    return window.__TAURI__ ? `asset://localhost${path}` : path;
  }

  function formatAddedAt(raw) {
    if (!raw) return '--';
    const d = new Date(raw);
    if (Number.isNaN(d.getTime())) return raw;
    const dd = String(d.getDate()).padStart(2, '0');
    const mm = String(d.getMonth() + 1).padStart(2, '0');
    const yy = String(d.getFullYear()).slice(-2);
    const hh = String(d.getHours()).padStart(2, '0');
    const min = String(d.getMinutes()).padStart(2, '0');
    return `${dd}-${mm}-${yy} | ${hh}:${min}`;
  }

  function isCompressedFilename(name) {
    const lower = String(name || '').toLowerCase();
    return ['.zip', '.7z', '.rar', '.tar', '.gz', '.bz2', '.xz'].some((ext) => lower.endsWith(ext));
  }

  function isDeployableByExtension(rowName, sourcePath) {
    const lowerName = String(rowName || '').trim().toLowerCase();
    const lowerSourceName = String(sourcePath || '')
      .split('/')
      .pop()
      .split('\\')
      .pop()
      .trim()
      .toLowerCase();
    return ['.pak', '.utoc', '.ucas'].some(
      (ext) => lowerName.endsWith(ext) || lowerSourceName.endsWith(ext),
    );
  }

  function mapModlistRows(listings) {
    return (listings || []).map((row) => ({
      modId: row.mod_id || 'unknown',
      name: row.name || row.mod_id || 'Unknown Mod',
      sourcePath: row.source_path || null,
      added: formatAddedAt(row.added_at),
      status: String(row.status || 'unknown').toUpperCase(),
      progress: Number(row.progress || 0),
      compressed: isCompressedFilename(row.name || row.mod_id || ''),
      deployable: !!row.deployable,
      deployerReason: row.deployer_reason || null,
    }));
  }

  async function loadModlistRows() {
    const listings = await invoke('get_modlist_listings', { appId: params.id });
    modlistRows = mapModlistRows(listings);
  }

  onMount(async () => {
    const steamPath = get(settings).steam_path || null;

    try {
      gameConfig = await invoke('get_game_config', { gameId: params.id });
    } catch (err) {
      addLog(`Failed to load game config: ${String(err)}`, 'error');
    }

    try {
      deployerCatalog = await invoke('get_available_deployers');
    } catch (err) {
      addLog(`Failed to load deployers: ${String(err)}`, 'error');
      deployerCatalog = [];
    }

    try {
      await invoke('ensure_game_cache', { appId: params.id, steamPath });
      const raw = await invoke('get_game_artwork', { appId: params.id, steamPath });
      artwork = {
        hero: toAssetUrl(raw.hero),
        logo: toAssetUrl(raw.logo),
        banner: toAssetUrl(raw.banner),
      };

      const configured = await invoke('get_configured_deployer', { appId: params.id });
      if (configured) {
        gameConfig = { ...gameConfig, deployer: configured };
      }
    } catch (err) {
      addLog(`Failed to load game assets: ${String(err)}`, 'error');
    }

    await refreshProfileModlist();

    try {
      await loadModlistRows();
    } catch (err) {
      addLog(`Failed to load modlist: ${String(err)}`, 'error');
      modlistRows = [];
    }
  });

  function onDone() {
    addLog('Returning to game manager view', 'info');
    history.back();
  }

  function onUpdateAll() {
    addLog('Update all requested', 'info');
  }

  function onLaunch() {
    addLog('Launch requested', 'info');
  }

  function onProfileChange(v) {
    if (!gameConfig.profiles?.length || v === '^ CREATE PROFILE') {
      addLog('Profile creation requested', 'info');
      return;
    }
    gameConfig = { ...gameConfig, active_profile: v };
    addLog(`Active profile set to ${v}`, 'success');
  }

  async function onDeployerChange(v) {
    const nextId = v === 'NONE' ? 'NONE' : (deployerIdByLabel.get(v) || v);
    try {
      await invoke('set_game_deployer', { appId: params.id, deployerId: nextId });
      gameConfig = { ...gameConfig, deployer: nextId === 'NONE' ? null : nextId };
      addLog(`Deployer set to ${v}`, 'success');
    } catch (err) {
      addLog(`Failed to set deployer: ${String(err)}`, 'error');
    }
  }

  function selectPaneTab(tabId) {
    activePaneTab = tabId;
  }

  async function refreshProfileModlist() {
    try {
      profileModRows = await invoke('get_profile_modlist', { appId: params.id });
    } catch (err) {
      addLog(`Failed to load profile mods: ${String(err)}`, 'error');
      profileModRows = [];
    }
  }

  async function onToggleProfileMod(row) {
    const nextEnabled = row.status !== 'ENABLED';
    try {
      await invoke('toggle_profile_mod', {
        appId: params.id,
        modId: row.mod_id,
        enabled: nextEnabled,
      });
      addLog(`${row.name} ${nextEnabled ? 'enabled' : 'disabled'}`, 'success');
      await refreshProfileModlist();
    } catch (err) {
      addLog(`Toggle failed: ${String(err)}`, 'error');
    }
  }

  async function onUncompressRow(row) {
    if (!row?.sourcePath) {
      addLog(`No source path available for ${row?.name || 'archive'}`, 'warning');
      return;
    }
    try {
      await invoke('uncompress_archive', { archivePath: row.sourcePath });
      addLog(`Uncompress complete: ${row.name}`, 'success');
      await loadModlistRows();
    } catch (err) {
      addLog(`Uncompress failed: ${String(err)}`, 'error');
    }
  }

  async function onDeployListing(row) {
    try {
      await invoke('deploy_listing', { appId: params.id, listingId: row.modId });
      addLog(`Deployed listing: ${row.name}`, 'success');
      await loadModlistRows();
      await refreshProfileModlist();
    } catch (err) {
      addLog(`Deploy failed: ${String(err)}`, 'error');
    }
  }

  async function scrollLogToBottom() {
    await tick();
    if (logViewport) {
      logViewport.scrollTop = logViewport.scrollHeight;
    }
  }
</script>

<section class="page profile-page">
  <TopBar backRoute={`/game/${params.id}`} />

  <main class="hero-strip">
    <div class="hero-left">
      {#if artwork.logo}
        <img src={artwork.logo} alt={game?.name || gameConfig.name || params.id} on:error={() => (artwork = { ...artwork, logo: '' })} />
      {:else}
        <div class="hero-name">[ {game?.name || gameConfig.name || params.id} ]</div>
      {/if}
    </div>
    <div class="hero-right">
      <button class="btn-primary launch-btn" on:click={onLaunch}>-- LAUNCH</button>
    </div>
  </main>

  <section class="action-bar">
    <div class="action-left">
      <button class="edit-btn done-btn" on:click={onDone}>// DONE</button>
      <button class="edit-btn update-all-btn" on:click={onUpdateAll}>^^ UPDATE ALL</button>
    </div>

    <div class="selectors-right">
      <div class="selector">
        <span class="selector-label">PROFILE :</span>
        <div class="selector-input">
          <Dropdown options={profileOptions} value={activeProfileValue} onChange={onProfileChange} />
        </div>
      </div>

      <div class="divider"></div>

      <div class="selector">
        <span class="selector-label">DEPLOYER =</span>
        <div class="selector-input">
          <Dropdown options={deployerOptions} value={deployerValue} onChange={onDeployerChange} />
        </div>
      </div>
    </div>
  </section>

  <section class="tab-bar">
    <div class="mods-tabs">
      {#each leftTabs as tab}
        <button class={`pane-tab ${activePaneTab === tab.id ? 'active' : ''}`} on:click={() => selectPaneTab(tab.id)}>{tab.label}</button>
      {/each}
    </div>
    <button class={`pane-tab ${activePaneTab === defaultProfileTab.id ? 'active' : ''}`} on:click={() => selectPaneTab(defaultProfileTab.id)}>{defaultProfileTab.label}</button>
  </section>

  <section class="modlist-pane-wrap">
    {#if activePaneTab === 'DEFAULT_PROFILE'}
      <article class="modlist-pane">
        <h3>MODS PLAIN AND SIMPLE</h3>
        <div class="mods-table">
          <div class="mods-table-head">
            <div class="mods-col mods-col-name">MOD NAME</div>
            <div class="mods-col mods-col-date">DATE ADDED</div>
            <div class="mods-col mods-col-status">STATUS</div>
          </div>
          {#each profileModRows as row}
            <div class="mods-table-row">
              <div class="mods-col mods-col-name">{row.name}</div>
              <div class="mods-col mods-col-date">{row.date_added}</div>
              <div class="mods-col mods-col-status">
                {#if row.toggleable}
                  <button
                    class={`status-toggle ${row.status === 'ENABLED' ? 'on' : 'off'}`}
                    on:click={() => onToggleProfileMod(row)}
                  >
                    <span class="toggle-pill"></span>
                  </button>
                {:else}
                  {row.status}
                {/if}
              </div>
            </div>
          {/each}
          {#if !profileModRows.length}
            <div class="mods-table-row mods-empty-row">
              <div class="mods-col mods-col-name">No profile mod entries found.</div>
              <div class="mods-col mods-col-date">--</div>
              <div class="mods-col mods-col-status">--</div>
            </div>
          {/if}
        </div>
      </article>
    {:else if activePaneTab === 'DOWNLOADED_MODS'}
      <article class="modlist-pane">
        <h3>MODS PLAIN AND SIMPLE</h3>
        <div class="mods-table">
          <div class="mods-table-head">
            <div class="mods-col mods-col-name">MOD NAME</div>
            <div class="mods-col mods-col-date">DATE ADDED</div>
            <div class="mods-col mods-col-status">STATUS</div>
          </div>
          {#each modlistRows as row}
            <div class="mods-table-row">
              <div class="mods-col mods-col-name">{row.name}</div>
              <div class="mods-col mods-col-date">{row.added}</div>
              <div class="mods-col mods-col-status">
                {#if row.status === 'DOWNLOADING'}
                  <div class="download-progress">
                    <div class="download-progress-label">{Math.round(Math.max(0, Math.min(1, row.progress)) * 100)}%</div>
                    <div class="download-progress-track">
                      <div class="download-progress-fill" style={`width:${Math.round(Math.max(0, Math.min(1, row.progress)) * 100)}%`}></div>
                    </div>
                  </div>
                {:else if row.compressed}
                  <button class="uncompress-btn" on:click={() => onUncompressRow(row)}>UNCOMPRESS</button>
                {:else if isDeployableByExtension(row.name, row.sourcePath)}
                  <button class="deploy-btn" on:click={() => onDeployListing(row)}>DEPLOY</button>
                {:else if row.deployable}
                  <button class="deploy-btn" on:click={() => onDeployListing(row)}>DEPLOY</button>
                {:else}
                  {row.status}
                {/if}
              </div>
            </div>
          {/each}
          {#if !modlistRows.length}
            <div class="mods-table-row mods-empty-row">
              <div class="mods-col mods-col-name">No listings found for this game id.</div>
              <div class="mods-col mods-col-date">--</div>
              <div class="mods-col mods-col-status">--</div>
            </div>
          {/if}
        </div>
      </article>
    {:else}
      <article class="modlist-pane">
        <h3>COLLECTIONS</h3>
        <p>COLLECTIONS PLACEHOLDER</p>
      </article>
    {/if}
  </section>

  <section class={`log-panel ${logExpanded ? 'expanded' : 'collapsed'}`}>
    <div class="log-head">
      <div class="preview">
        {#each $sessionLog.slice(-3) as entry}
          <div class={`line ${entry.type || 'default'}`}>
            <span class="time">{entry.time}</span><span class="pipe"> | </span><span>{entry.message}</span>
            {#if entry.progress !== undefined}
              <div class="log-progress">
                <div class="log-progress-fill" style={`width:${Math.max(0, Math.min(100, entry.progress))}%`}></div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
      <button class="toggle" on:click={() => (logExpanded = !logExpanded)}>{logExpanded ? '↑' : '↓'}</button>
    </div>

    {#if logExpanded}
      <div class="log-body" bind:this={logViewport}>
        {#each $sessionLog as entry}
          <div class={`line ${entry.type || 'default'}`}>
            <span class="time">{entry.time}</span><span class="pipe"> | </span><span>{entry.message}</span>
            {#if entry.progress !== undefined}
              <div class="log-progress">
                <div class="log-progress-fill" style={`width:${Math.max(0, Math.min(100, entry.progress))}%`}></div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>
</section>

<style>
  .profile-page {
    background: var(--bg);
    color: var(--text);
    font-family: var(--font);
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding-top: var(--topbar-height, 56px);
    box-sizing: border-box;
  }

  .hero-strip {
    height: 80px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
    border-bottom: var(--border-subtle);
    background: var(--bg-surface);
    box-sizing: border-box;
  }

  .hero-left img {
    max-height: 48px;
    max-width: 260px;
    object-fit: contain;
    display: block;
  }

  .hero-name {
    font-size: 20px;
    font-family: var(--font);
    color: var(--text);
  }

  .hero-right { width: 160px; }
  .launch-btn { width: 100%; height: 32px; }

  .action-bar {
    width: 100%;
    background: var(--bg-surface);
    border-bottom: var(--border-subtle);
    padding: 12px 24px;
    display: flex;
    align-items: center;
    gap: 24px;
    flex-shrink: 0;
    box-sizing: border-box;
  }

  .action-left { display: flex; align-items: center; gap: 12px; }

  .edit-btn {
    border: var(--border-subtle);
    background: transparent;
    color: var(--text);
    font-family: var(--font-ui);
    padding: 8px 12px;
    border-radius: var(--border-radius);
    cursor: pointer;
  }

  .done-btn { background: var(--action); color: #fff; border: 0; }
  .update-all-btn { background: var(--interactive); color: var(--btn-primary-text); border: 0; }

  .selectors-right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 24px;
  }

  .selector { display: flex; align-items: center; gap: 10px; }
  .selector-label { color: var(--text-muted); font-family: var(--font-ui); }
  .selector-input { width: 240px; }
  .divider { width: 1px; height: 20px; background: var(--ash); }

  .tab-bar {
    width: 100%;
    background: transparent;
    border-bottom: var(--border-subtle);
    padding: 0 24px;
    min-height: 58px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    box-sizing: border-box;
  }

  .mods-tabs {
    display: flex;
    align-items: center;
    gap: 24px;
    height: 100%;
  }

  .pane-tab {
    border: 1px solid transparent;
    background: transparent;
    color: var(--text);
    font-family: var(--font-ui);
    font-size: 1em;
    letter-spacing: 1px;
    text-transform: uppercase;
    cursor: pointer;
    padding: 10px 18px;
    border-radius: 0;
    line-height: 1;
    outline: none;
    box-shadow: none;
  }

  .pane-tab.active {
    border-color: var(--interactive);
    background: var(--bg-surface);
    color: var(--interactive);
    outline: none;
    box-shadow: none;
  }

  .pane-tab:hover {
    color: var(--action);
  }

  .pane-tab:focus,
  .pane-tab:focus-visible,
  .pane-tab:active {
    outline: none;
    box-shadow: none;
  }

  .modlist-pane-wrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--bg);
    border-bottom: var(--border-subtle);
    padding: 0;
    margin: 0;
  }

  .modlist-pane {
    margin: 0;
    min-height: 240px;
    border: var(--border-subtle);
    background: var(--bg-surface);
    padding: 24px;
    box-sizing: border-box;
    font-family: var(--font-ui);
  }

  .modlist-pane h3 {
    margin: 0 0 12px;
    color: var(--text);
    letter-spacing: 1px;
  }

  .mods-table {
    border-top: var(--border-subtle);
  }

  .mods-table-head,
  .mods-table-row {
    display: grid;
    grid-template-columns: 1fr 240px 180px;
    gap: 16px;
    padding: 10px 0;
    border-bottom: var(--border-subtle);
  }

  .mods-table-head {
    color: var(--text-muted);
  }

  .mods-empty-row {
    color: var(--text-muted);
    font-style: italic;
  }

  .mods-col-date {
    text-align: center;
  }

  .mods-col-status {
    text-align: right;
  }

  .download-progress {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  .download-progress-label {
    min-width: 34px;
    color: var(--interactive);
  }

  .download-progress-track {
    width: 90px;
    height: 7px;
    background: var(--ash);
    overflow: hidden;
  }

  .download-progress-fill {
    height: 100%;
    background: var(--interactive);
    transition: width 0.2s linear;
  }

  .uncompress-btn,
  .deploy-btn {
    border: 0;
    border-radius: var(--border-radius);
    padding: 4px 10px;
    font-family: var(--font-ui);
    cursor: pointer;
    background: var(--interactive);
    color: var(--btn-primary-text);
  }

  .uncompress-btn:hover,
  .deploy-btn:hover {
    opacity: 0.85;
  }

  .status-toggle {
    width: 112px;
    height: 32px;
    border: 2px solid var(--interactive);
    background: transparent;
    padding: 0 10px;
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    cursor: pointer;
  }

  .status-toggle .toggle-pill {
    width: 58px;
    height: 70%;
    background: var(--text-muted);
  }

  .status-toggle.on .toggle-pill {
    background: var(--interactive);
  }

  .modlist-pane p {
    margin: 0;
    color: var(--interactive);
    font-size: 18px;
  }

  .modlist-pane small {
    display: inline-block;
    margin-top: 10px;
    color: var(--text-muted);
  }

  .muted { color: var(--text-muted); }

  .btn-primary {
    border: 0;
    border-radius: var(--border-radius);
    padding: 6px 10px;
    font-family: var(--font-ui);
    cursor: pointer;
    background: var(--interactive);
    color: var(--btn-primary-text);
  }

  .btn-primary:hover { opacity: 0.85; }

  .log-panel {
    background: var(--bg-surface);
    border-top: var(--border-subtle);
    flex-shrink: 0;
    height: calc(3 * 1.6em + 16px);
  }

  .log-panel.expanded {
    max-height: 60vh;
    height: 60vh;
    border: var(--border-subtle);
    background: var(--bg);
  }

  .log-head {
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    box-sizing: border-box;
  }

  .log-panel.expanded .log-head {
    border-bottom: var(--border-subtle);
  }

  .preview { flex: 1; min-width: 0; overflow: hidden; }

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
    height: calc(60vh - 36px);
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

  .log-progress {
    margin-top: 4px;
    height: 6px;
    width: 220px;
    background: var(--ash);
    overflow: hidden;
  }

  .log-progress-fill {
    height: 100%;
    background: var(--interactive);
    transition: width 0.2s linear;
  }

  .time { color: var(--text-muted); }
  .pipe { color: var(--ash); }

  .line.default { color: var(--text); }
  .line.success { color: var(--success); }
  .line.error { color: var(--action); }
  .line.warning { color: var(--warning); }
  .line.info { color: var(--interactive); }
</style>
