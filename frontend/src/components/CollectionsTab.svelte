<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { get } from 'svelte/store';
  import { settings } from '../stores/settings.js';
  import { addLog } from '../stores/log.js';

  export let appId;
  export let gameDomain = null;
  export let onInstalled = async () => {};

  let collections = [];
  let loading = true;
  let message = '';
  let selected = null;
  let installing = false;

  $: nexusApiKey = get(settings)?.accounts?.nexus_api_key || '';
  $: hasNexusCollections = collections.some((collection) => collection.source === 'nexus');

  function typeLabel(collection) {
    return collection.source === 'nexus' ? 'NEXUS' : 'PACK';
  }

  function formatSize(bytes) {
    if (!bytes) return '';
    const units = ['B', 'KB', 'MB', 'GB'];
    let value = bytes;
    let idx = 0;
    while (value >= 1024 && idx < units.length - 1) {
      value /= 1024;
      idx += 1;
    }
    return `${value.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
  }

  function formatPackType(type) {
    return String(type || '')
      .replace(/_/g, ' ')
      .toUpperCase();
  }

  async function loadCollections() {
    collections = await invoke('list_collections', { appId });
  }

  async function maybeFetchNexus() {
    if (!nexusApiKey) {
      message = '{ NO NEXUS API KEY } add a nexus api key in settings to browse collections';
      return;
    }
    if (!gameDomain) {
      message = '{ NO NEXUS GAME DOMAIN } add game_domain to this game config to browse nexus collections';
      return;
    }
    if (hasNexusCollections) return;

    try {
      await invoke('fetch_nexus_collections', { appId, gameDomain });
      await loadCollections();
    } catch (err) {
      addLog(`Nexus collections fetch failed: ${String(err)}`, 'warning');
    }
  }

  async function installSelected() {
    if (!selected) return;
    installing = true;
    try {
      if (selected.source === 'caldera') {
        addLog(`Importing collection pack: ${selected.name}`, 'info');
        const result = await invoke('import_pack', { packPath: selected.pack_path });
        addLog(`Collection import complete: ${result.mods_installed} installed, ${result.mods_queued} queued, ${result.mods_failed} failed`, 'success');
        await onInstalled();
      } else {
        addLog(`Nexus Collection install stubbed: ${selected.name} - coming with download pipeline`, 'warning');
      }
      selected = null;
    } catch (err) {
      addLog(`Collection install failed: ${String(err)}`, 'error');
    } finally {
      installing = false;
    }
  }

  function closeModalFromBackdrop(event) {
    if (event.currentTarget === event.target) {
      selected = null;
    }
  }

  onMount(async () => {
    loading = true;
    try {
      await loadCollections();
      await maybeFetchNexus();
    } catch (err) {
      addLog(`Failed to load collections: ${String(err)}`, 'warning');
    } finally {
      loading = false;
    }
  });
</script>

<article class="collections-tab">
  <div class="collections-head">
    <h3>COLLECTIONS</h3>
    {#if message}
      <div class="collections-message">{message}</div>
    {/if}
  </div>

  <div class="collections-table">
    <div class="collections-row collections-table-head">
      <div>TYPE</div>
      <div>NAME</div>
      <div>MODS</div>
      <div>SIZE</div>
      <div></div>
    </div>

    {#each collections as collection}
      <div class="collections-row">
        <div><span class={`type-badge ${collection.source}`}>{`{ ${typeLabel(collection)} }`}</span></div>
        <div>{collection.name}</div>
        <div>{collection.mod_count || 0}</div>
        <div>{formatSize(collection.size_bytes)}</div>
        <div><button class="install-btn" on:click={() => (selected = collection)}>INSTALL ▶</button></div>
      </div>
    {/each}

    {#if !loading && !collections.length}
      <div class="collections-empty">
        <div>{'{ NO COLLECTIONS }'}</div>
        <p>import a .caldera pack or browse nexus collections above</p>
      </div>
    {/if}
  </div>
</article>

{#if selected}
  <div class="collection-modal-backdrop" role="button" tabindex="0" on:click={closeModalFromBackdrop} on:keydown={(event) => event.key === 'Escape' && (selected = null)}>
    <section class="collection-modal" role="dialog" aria-modal="true" tabindex="-1">
      <h2>INSTALL COLLECTION</h2>
      <div class="modal-title-row">
        <span class={`type-badge ${selected.source}`}>{`{ ${typeLabel(selected)} }`}</span>
        <strong>{selected.name}</strong>
        {#if selected.version}<span>v{selected.version}</span>{/if}
      </div>
      <p class="modal-summary">
        {selected.mod_count || 0} mods{#if selected.size_bytes} · {formatSize(selected.size_bytes)}{/if}{#if selected.pack_type} · {formatPackType(selected.pack_type)} INSTALL PACK{/if}
      </p>

      {#if selected.source === 'nexus'}
        <p>{selected.description || 'No description available.'}</p>
        <p>{selected.endorsements || 0} endorsements</p>
        <p class="stub-warning">nexus collection install is not yet implemented - mods will be queued when the download pipeline lands</p>
      {:else}
        <p class="stub-warning">pack mod preview is not stored in the collection entry yet; install will import the saved .caldera pack</p>
      {/if}

      <div class="modal-actions">
        <button on:click={() => (selected = null)}>CANCEL</button>
        <button class="install-btn" disabled={installing} on:click={installSelected}>INSTALL ▶</button>
      </div>
    </section>
  </div>
{/if}

<style>
  .collections-tab {
    padding: 24px;
    color: var(--text);
  }

  .collections-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 16px;
  }

  .collections-head h3 {
    margin: 0;
  }

  .collections-message {
    color: var(--text-muted);
    font-family: var(--font-ui), monospace;
    font-size: 13px;
  }

  .collections-table {
    border-top: 1px solid var(--ash);
  }

  .collections-row {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr) 80px 120px 120px;
    gap: 14px;
    align-items: center;
    border-bottom: 1px solid var(--ash);
    padding: 12px 0;
  }

  .collections-table-head {
    color: var(--text-muted);
    font-family: var(--font-ui), monospace;
    letter-spacing: 1px;
  }

  .type-badge {
    display: inline-block;
    border: 1px solid var(--text-muted);
    color: var(--text-muted);
    padding: 3px 8px;
    font-family: var(--font-ui), monospace;
    font-size: 12px;
  }

  .type-badge.caldera {
    border-color: #E8B84B;
    color: #E8B84B;
  }

  .install-btn,
  .modal-actions button {
    border: 1px solid #E8B84B;
    background: transparent;
    color: #E8B84B;
    padding: 8px 12px;
    font-family: var(--font-ui), monospace;
    cursor: pointer;
  }

  .install-btn:hover,
  .modal-actions button:hover {
    background: #E8B84B;
    color: #1A1614;
  }

  .collections-empty {
    padding: 30px 0;
    color: var(--text-muted);
    font-family: var(--font-ui), monospace;
  }

  .collections-empty p {
    margin: 8px 0 0;
  }

  .collection-modal-backdrop {
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

  .collection-modal {
    width: min(560px, 100%);
    background: #221E1B;
    border: 1px solid #E8B84B;
    padding: 28px;
    color: var(--text);
    box-sizing: border-box;
  }

  .collection-modal h2 {
    margin: 0 0 18px;
    color: #E8B84B;
  }

  .modal-title-row,
  .modal-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .modal-summary,
  .stub-warning {
    color: var(--text-muted);
  }

  .modal-actions {
    justify-content: space-between;
    margin-top: 24px;
  }
</style>
