<script>
  import { onDestroy, onMount, tick } from 'svelte';
  import { push } from 'svelte-spa-router';
  import { invoke } from '@tauri-apps/api/core';
  import { documentDir } from '@tauri-apps/api/path';
  import { open } from '@tauri-apps/plugin-dialog';
  import { get } from 'svelte/store';
  import TopBar from '../components/TopBar.svelte';
  import Button from '../components/Button.svelte';
  import CollectionsTab from '../components/CollectionsTab.svelte';
  import Dropdown from '../components/Dropdown.svelte';
  import Dropup from '../components/Dropup.svelte';
  import { gameList } from '../stores/game.js';
  import { settings } from '../stores/settings.js';
  import { sessionLog, addLog } from '../stores/log.js';
  import { showToast } from '../components/Toast.svelte';

  export let params = {};

  let deployerCatalog = [];
  let game = null;
  let gameConfig = {
    game_id: '',
    name: '',
    mod_directory: '',
    game_domain: null,
    deployer: null,
    active_profile: null,
    profiles: [],
  };
  let artwork = { hero: '', logo: '' };
  let logExpanded = false;
  let logViewport;
  let activePaneTab = 'DOWNLOADED_MODS';
  let modlistRows = [];
  let showExportModal = false;
  let exportPackVersion = '';
  let exportPackName = '';
  let exportPackType = 'OFFLINE INSTALL PACK';
  let exportPath = '';
  let includeDisabledMods = false;
  let exportVersionError = '';
  let exportNameError = '';
  let unlistenSessionLog = null;
  const leftTabs = [
    { id: 'DOWNLOADED_MODS', label: 'DOWNLOADED MODS' },
    { id: 'COLLECTIONS', label: 'COLLECTIONS' },
  ];
  const defaultProfileTab = { id: 'DEFAULT_PROFILE', label: 'DEFAULT PROFILE' };
  const exportPackTypes = [
    'OFFLINE INSTALL PACK',
    'ONLINE INSTALL PACK',
    'ONLINE INSTALL + LOCAL',
  ];

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
  $: deployerValue = !gameConfig.deployer || gameConfig.deployer === 'NONE'
    ? 'SELECT DEPLOYER'
    : deployerLabelById.get(gameConfig.deployer) || gameConfig.deployer;
  $: lastLog = $sessionLog.length ? $sessionLog[$sessionLog.length - 1] : { time: '--:--', message: 'No log entries yet', type: 'default' };

  $: if (logExpanded) {
    scrollLogToBottom();
  }

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
      showToast(String(err), 'error');
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
      addLog(`Failed to load artwork: ${String(err)}`, 'error');
    }

    try {
      await loadModlistRows();
    } catch (err) {
      addLog(`Failed to load modlist listings: ${String(err)}`, 'warning');
      modlistRows = [];
    }

    unlistenSessionLog = sessionLog.subscribe((entries) => {
      const last = entries[entries.length - 1];
      if (
        !last?.message?.startsWith('Download queued:')
        && !last?.message?.startsWith('Download complete:')
        && !last?.message?.startsWith('Download failed:')
        && !last?.progressKey?.startsWith('download:')
      ) return;
      loadModlistRows().catch((err) => {
        addLog(`Failed to refresh modlist listings: ${String(err)}`, 'warning');
      });
    });

    addLog('"CALDERA" initialised successfully', 'success');
    addLog(`Loading game: "${game?.name || gameConfig.name || params.id}"`, 'info');
    if (!gameConfig.deployer || gameConfig.deployer === 'NONE') {
      addLog('No deployer configured — select one to get started', 'warning');
    } else {
      const deployerName = deployerLabelById.get(gameConfig.deployer) || gameConfig.deployer;
      addLog(`Deployer active: ${deployerName}`, 'success');
    }
  });

  function onSetup() {
    showToast('Setup wizard coming soon', 'info');
    addLog('Setup wizard requested', 'info');
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

  function openProfilePage() {
    push(`/game/${params.id}/profile`);
  }

  async function openExportPackModal() {
    exportVersionError = '';
    exportNameError = '';
    showExportModal = true;

    if (!exportPath.trim()) {
      try {
        exportPath = await documentDir();
      } catch (err) {
        addLog(`Failed to resolve Documents folder: ${String(err)}`, 'warning');
      }
    }
  }

  function closeExportPackModal() {
    showExportModal = false;
    exportVersionError = '';
    exportNameError = '';
  }

  function onExportBackdropClick(event) {
    if (event.currentTarget === event.target) {
      closeExportPackModal();
    }
  }

  function onExportBackdropKeydown(event) {
    if (event.key === 'Escape' || event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      closeExportPackModal();
    }
  }

  async function browseExportPath() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === 'string' && selected.length > 0) {
        exportPath = selected;
      }
    } catch (err) {
      showToast(`Folder picker failed: ${String(err)}`, 'error');
    }
  }

  async function submitExportPack() {
    const trimmedVersion = exportPackVersion.trim();
    const trimmedName = exportPackName.trim();
    if (!trimmedVersion) {
      exportVersionError = 'Version is required';
      return;
    }
    if (!/^\d+\.\d+\.\d+$/.test(trimmedVersion)) {
      exportVersionError = 'Version must use semver format, e.g. 1.0.0';
      return;
    }
    if (!trimmedName) {
      exportNameError = 'Pack name is required';
      return;
    }

    if (!exportPath.trim()) {
      try {
        exportPath = await documentDir();
      } catch (err) {
        addLog(`Failed to resolve Documents folder: ${String(err)}`, 'warning');
      }
    }

    exportVersionError = '';
    exportNameError = '';
    addLog(`Exporting pack: ${trimmedName} (${exportPackType})`, 'info');

    const packType = {
      'OFFLINE INSTALL PACK': 'offline',
      'ONLINE INSTALL PACK': 'online',
      'ONLINE INSTALL + LOCAL': 'online_local',
    }[exportPackType];

    await invoke('export_pack', {
      appId: params.id,
      profileName: activeProfileValue,
      packName: trimmedName,
      version: trimmedVersion,
      packType,
      exportPath,
      includeDisabled: includeDisabledMods,
    });

    closeExportPackModal();
  }

  async function importPack() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Caldera pack', extensions: ['caldera'] }],
      });
      if (typeof selected !== 'string' || !selected.length) return;

      addLog(`Importing pack: ${selected}`, 'info');
      const result = await invoke('import_pack', { packPath: selected });
      addLog(`Import complete: ${result.mods_installed} installed, ${result.mods_queued} queued, ${result.mods_failed} failed`, 'success');
      await loadModlistRows();
    } catch (err) {
      addLog(`Import failed: ${String(err)}`, 'error');
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

  onDestroy(() => {
    if (unlistenSessionLog) unlistenSessionLog();
  });
</script>

<section class="page game-detail">
  <TopBar backRoute="/" />

  <main class="hero-section">
    <section class="hero">
      {#if artwork.hero}
        <img
          class="hero-image"
          src={artwork.hero}
          alt={game?.name || gameConfig.name || params.id}
          on:error={() => (artwork = { ...artwork, hero: '' })}
        />
      {:else}
        <div class="hero-fallback">
          <div class="hero-placeholder-name">{game?.name || gameConfig.name || params.id}</div>
        </div>
      {/if}

      <div class="hero-logo">
        {#if artwork.logo}
          <img
            src={artwork.logo}
            alt={game?.name || gameConfig.name || params.id}
            on:error={() => (artwork = { ...artwork, logo: '' })}
          />
        {:else}
          <div class="hero-name">{game?.name || gameConfig.name || params.id}</div>
        {/if}
      </div>

      <div class="hero-setup">
        <Button variant="secondary" label="// SETUP" onClick={onSetup} />
      </div>
    </section>
  </main>

  <section class="action-bar">
    <div class="action-left">
      <button class="edit-btn" on:click={openProfilePage}>// EDIT</button>
    </div>

    <div class="selectors-right">
      <div class="selector">
        <span class="selector-label">PROFILE :</span>
        <div class="selector-input">
          <Dropup options={profileOptions} value={activeProfileValue} onChange={onProfileChange} />
        </div>
      </div>

      <div class="divider"></div>

      <div class="selector">
        <span class="selector-label">DEPLOYER =</span>
        <div class="selector-input">
          <Dropup options={deployerOptions} value={deployerValue} onChange={onDeployerChange} />
        </div>
      </div>
    </div>
  </section>

  <section class="export-bar">
    <button class="import-open-btn" on:click={importPack}>IMPORT ⭳</button>
    <button class="export-open-btn" on:click={openExportPackModal}>EXPORT ⭲</button>
  </section>

  <section class="tab-bar">
    <div class="mods-tabs">
      {#each leftTabs as tab}
        <button class={`pane-tab ${activePaneTab === tab.id ? 'active' : ''}`} on:click={() => selectPaneTab(tab.id)}>{tab.label}</button>
      {/each}
    </div>
    <button class={`pane-tab ${activePaneTab === defaultProfileTab.id ? 'active' : ''}`} on:click={openProfilePage}>{defaultProfileTab.label}</button>
  </section>

  <section class="modlist-pane-wrap">
    {#if activePaneTab === 'DOWNLOADED_MODS'}
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
      <CollectionsTab appId={params.id} gameDomain={gameConfig.game_domain} onInstalled={loadModlistRows} />
    {/if}
  </section>

  {#if showExportModal}
    <div
      class="export-modal-backdrop"
      role="button"
      tabindex="0"
      aria-label="Close export pack modal"
      on:click={onExportBackdropClick}
      on:keydown={onExportBackdropKeydown}
    >
      <section
        class="export-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="export-pack-title"
        tabindex="-1"
      >
        <button class="export-close-btn" aria-label="Close export pack modal" on:click={closeExportPackModal}>X</button>
        <h2 id="export-pack-title">EXPORT PACK</h2>

        <label class="export-field">
          <span>&#123; VERSION &#125;</span>
          <input
            type="text"
            bind:value={exportPackVersion}
            placeholder="1.0.0"
            class:error={exportVersionError}
            on:input={() => (exportVersionError = '')}
          />
          {#if exportVersionError}
            <div class="export-error">{exportVersionError}</div>
          {/if}
        </label>

        <label class="export-field">
          <span>&#123; PACK NAME &#125;</span>
          <input
            type="text"
            bind:value={exportPackName}
            placeholder="PACK NAME"
            class:error={exportNameError}
            on:input={() => (exportNameError = '')}
          />
          {#if exportNameError}
            <div class="export-error">{exportNameError}</div>
          {/if}
        </label>

        <div class="export-field">
          <span>&#123; PACK TYPE &#125;</span>
          <div class="export-type-list">
            {#each exportPackTypes as type}
              <button
                class={`export-type-btn ${exportPackType === type ? 'active' : ''}`}
                on:click={() => (exportPackType = type)}
              >{type}</button>
            {/each}
          </div>
        </div>

        <label class="export-field">
          <span>&#123; EXPORT PATH &#125;</span>
          <div class="export-path-row">
            <input type="text" bind:value={exportPath} placeholder="Documents" />
            <button class="export-browse-btn" type="button" on:click={browseExportPath}>...</button>
          </div>
        </label>

        <label class="export-toggle-row">
          <span>&#123; INCLUDE DISABLED MODS &#125;</span>
          <button
            class={`export-toggle ${includeDisabledMods ? 'on' : ''}`}
            type="button"
            on:click={() => (includeDisabledMods = !includeDisabledMods)}
          >{includeDisabledMods ? 'ON' : 'OFF'}</button>
        </label>

        <button class="export-submit-btn" on:click={submitExportPack}>EXPORT ⭲</button>
      </section>
    </div>
  {/if}

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
  .game-detail {
    background: var(--bg);
    color: var(--text);
    font-family: var(--font);
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding-top: var(--topbar-height, 56px);
    box-sizing: border-box;
  }

  .hero-section {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  .hero {
    position: relative;
    height: 100%;
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
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .hero-placeholder-name {
    color: var(--text-muted);
    font-family: var(--font);
    text-transform: uppercase;
  }

  .hero-logo {
    position: absolute;
    left: 24px;
    bottom: 24px;
    z-index: 2;
  }

  .hero-logo img {
    max-height: 64px;
    max-width: 240px;
    object-fit: contain;
    display: block;
  }

  .hero-name {
    font-size: 24px;
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
    flex-shrink: 0;
    box-sizing: border-box;
  }

  .action-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .edit-btn {
    border: var(--border-subtle);
    background: transparent;
    color: var(--text);
    font-family: var(--font-ui);
    padding: 8px 12px;
    border-radius: var(--border-radius);
    cursor: pointer;
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

  .selectors-right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 24px;
  }

  .selector-label {
    color: var(--text-muted);
    font-family: var(--font-ui);
  }

  .selector-input {
    width: 240px;
  }

  .export-bar {
    width: 100%;
    background: #221E1B;
    border-bottom: 1px solid rgba(232, 184, 75, 0.28);
    padding: 10px 24px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    box-sizing: border-box;
  }

  .import-open-btn,
  .export-open-btn,
  .export-submit-btn,
  .export-browse-btn,
  .export-close-btn,
  .export-type-btn,
  .export-toggle {
    font-family: var(--font-ui), monospace;
    text-transform: uppercase;
    letter-spacing: 1px;
    border-radius: 2px;
    cursor: pointer;
  }

  .import-open-btn,
  .export-open-btn,
  .export-submit-btn {
    border: 1px solid #E8B84B;
    background: transparent;
    color: #E8B84B;
    padding: 9px 16px;
  }

  .import-open-btn:hover,
  .export-open-btn:hover,
  .export-submit-btn:hover,
  .export-browse-btn:hover,
  .export-close-btn:hover,
  .export-type-btn:hover,
  .export-toggle:hover {
    color: #1A1614;
    background: #E8B84B;
  }

  .export-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    box-sizing: border-box;
  }

  .export-modal {
    position: relative;
    width: min(620px, 100%);
    background: #221E1B;
    border: 1px solid #E8B84B;
    color: var(--text);
    font-family: var(--font-ui), monospace;
    padding: 28px;
    box-sizing: border-box;
    border-radius: 2px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.55);
  }

  .export-modal h2 {
    margin: 0 0 22px;
    color: #E8B84B;
    font-size: 22px;
    letter-spacing: 2px;
    font-weight: 400;
  }

  .export-close-btn {
    position: absolute;
    top: 16px;
    right: 16px;
    border: 1px solid #E8B84B;
    background: transparent;
    color: #E8B84B;
    width: 32px;
    height: 32px;
  }

  .export-field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 18px;
  }

  .export-field span,
  .export-toggle-row span {
    color: var(--text-muted);
    letter-spacing: 1px;
    font-size: 13px;
  }

  .export-field input {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid #E8B84B;
    border-radius: 2px;
    background: #1A1614;
    color: var(--text);
    font-family: var(--font-ui), monospace;
    padding: 10px 12px;
    outline: none;
  }

  .export-field input:focus {
    border-color: #C0392B;
  }

  .export-field input.error {
    border-color: #C0392B;
  }

  .export-error {
    color: #C0392B;
    font-size: 12px;
    letter-spacing: 1px;
  }

  .export-type-list {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .export-type-btn {
    min-height: 44px;
    border: 1px solid #E8B84B;
    background: #1A1614;
    color: #E8B84B;
    padding: 8px 10px;
  }

  .export-type-btn.active {
    border-color: #C0392B;
    background: #C0392B;
    color: #fff;
  }

  .export-path-row {
    display: grid;
    grid-template-columns: 1fr 48px;
    gap: 8px;
  }

  .export-browse-btn,
  .export-toggle {
    border: 1px solid #E8B84B;
    background: #1A1614;
    color: #E8B84B;
  }

  .export-toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin: 6px 0 24px;
  }

  .export-toggle {
    min-width: 68px;
    padding: 8px 12px;
  }

  .export-toggle.on {
    border-color: #C0392B;
    background: #C0392B;
    color: #fff;
  }

  .export-submit-btn {
    width: 100%;
  }

  @media (max-width: 720px) {
    .export-type-list {
      grid-template-columns: 1fr;
    }

    .export-modal {
      padding: 22px;
    }
  }

  .tab-bar {
    width: 100%;
    background: var(--bg-surface);
    border-bottom: var(--border-subtle);
    padding: 10px 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    box-sizing: border-box;
  }

  .mods-tabs,
  .profile-tabs {
    display: flex;
    align-items: center;
    gap: 10px;
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

  .uncompress-btn {
    border: 0;
    border-radius: var(--border-radius);
    padding: 4px 10px;
    font-family: var(--font-ui);
    cursor: pointer;
    background: var(--interactive);
    color: var(--btn-primary-text);
  }

  .uncompress-btn:hover {
    opacity: 0.85;
  }

  .deploy-btn {
    border: 0;
    border-radius: var(--border-radius);
    padding: 4px 10px;
    font-family: var(--font-ui);
    cursor: pointer;
    background: var(--interactive);
    color: var(--btn-primary-text);
  }

  .deploy-btn:hover {
    opacity: 0.85;
  }

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
    border-bottom: 0;
    box-sizing: border-box;
  }

  .log-panel.expanded .log-head {
    border-bottom: var(--border-subtle);
  }

  .preview {
    flex: 1;
    min-width: 0;
    overflow: hidden;
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
