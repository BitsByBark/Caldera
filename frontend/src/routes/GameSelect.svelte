<script>
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { push } from 'svelte-spa-router';
  import { open } from '@tauri-apps/plugin-dialog';
  import Button from '../components/Button.svelte';
  import TopBar from '../components/TopBar.svelte';
  import { gameList, currentGame } from '../stores/game';
  import { settings } from '../stores/settings.js';
  import { getSteamGames, getGameArtwork, addManualGame } from '../lib/tauri';
  import { showToast } from '../components/Toast.svelte';

  let artworkById = {};
  let posterLoadFailed = {};
  let showAddModal = false;
  let manualGameName = '';
  let manualInstallPath = '';
  let addingManualGame = false;

  async function refreshGames() {
    const steamPath = get(settings).steam_path || null;
    const games = await getSteamGames(steamPath);
    gameList.set(games);
    for (const game of games) {
      artworkById[game.app_id] = await getGameArtwork(game.app_id, steamPath);
    }
  }

  onMount(async () => {
    try {
      await refreshGames();
      if (get(gameList).length > 0) {
        showToast(`Found ${get(gameList).length} games`, 'success');
      } else {
        showToast('No games found in Steam library.', 'warning');
      }
    } catch (err) {
      showToast(String(err), 'error');
      gameList.set([]);
    }
  });

  function openGame(game) {
    currentGame.set(game);
    push(`/game/${game.app_id}`);
  }

  function markPosterFailed(appId) {
    posterLoadFailed = { ...posterLoadFailed, [appId]: true };
  }

  function openAddGameModal() {
    manualGameName = '';
    manualInstallPath = '';
    showAddModal = true;
  }

  function closeAddGameModal() {
    if (addingManualGame) return;
    showAddModal = false;
  }

  async function submitManualGame() {
    if (addingManualGame) return;
    addingManualGame = true;
    try {
      await addManualGame(manualGameName, manualInstallPath);
      await refreshGames();
      showToast(`Added game "${manualGameName.trim()}"`, 'success');
      showAddModal = false;
    } catch (err) {
      showToast(String(err), 'error');
    } finally {
      addingManualGame = false;
    }
  }

  async function chooseInstallDirectory() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (typeof selected === 'string' && selected.length > 0) {
        manualInstallPath = selected;
      }
    } catch (err) {
      showToast(`Directory picker failed: ${String(err)}. Restart CALDERA and try again.`, 'error');
    }
  }
</script>

<section class="page">
  <TopBar />

  <main class="content">
    <div class="grid">
      {#each $gameList as game}
        <button class="card" on:click={() => openGame(game)}>
          <div class="poster-frame">
            {#if artworkById[game.app_id]?.banner && !posterLoadFailed[game.app_id]}
              <img
                class="poster"
                src={artworkById[game.app_id].banner}
                alt={game.name}
                on:error={() => markPosterFailed(game.app_id)}
              />
            {:else}
              <div class="poster poster-placeholder"></div>
            {/if}
            <div class="title-overlay">"{game.name}"</div>
          </div>
        </button>
      {/each}
    </div>
  </main>

  <div class="bottombar">
    <div class="bottombar-inner">
      <div class="add"><Button variant="secondary" icon="+" label="ADD GAME" onClick={openAddGameModal} /></div>
    </div>
  </div>

  {#if showAddModal}
    <div class="modal-backdrop" on:click={closeAddGameModal}>
      <div class="modal" on:click|stopPropagation>
        <div class="modal-title">ADD MANUAL GAME</div>
        <label class="field">
          <span>GAME NAME</span>
          <input type="text" bind:value={manualGameName} placeholder="Ready or Not" />
        </label>
        <label class="field">
          <span>INSTALL DIRECTORY (OPTIONAL)</span>
          <div class="path-row">
            <input type="text" bind:value={manualInstallPath} placeholder="leave blank to add later" />
            <button class="pick-btn" type="button" on:click={chooseInstallDirectory}>...</button>
          </div>
        </label>
        <div class="modal-actions">
          <button class="modal-btn cancel" on:click={closeAddGameModal} disabled={addingManualGame}>CANCEL</button>
          <button
            class="modal-btn add"
            on:click={submitManualGame}
            disabled={addingManualGame || !manualGameName.trim()}
          >ADD</button>
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .page {
    min-height: 100vh;
    background: var(--bg);
  }

  .content {
    padding: 76px 32px 76px;
  }

  .grid {
    display: grid;
    gap: 14px;
    grid-template-columns: repeat(5, 1fr);
  }

  .card {
    border: 1px solid transparent;
    background: var(--bg-surface);
    text-align: left;
    padding: 6px;
    cursor: pointer;
    font-family: var(--font);
    border-radius: var(--border-radius);
  }

  .card:hover {
    border: var(--border);
    background: var(--bg-hover);
  }

  .poster-frame {
    width: 100%;
    aspect-ratio: 2 / 3;
    overflow: hidden;
    background: var(--bg-surface);
    position: relative;
    border-radius: var(--border-radius);
  }

  .poster {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .poster-placeholder {
    background: var(--ash);
  }

  .title-overlay {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(26,22,20,0.85);
    color: var(--text);
    padding: 6px 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border-radius: 0 0 var(--border-radius) var(--border-radius);
  }

  .bottombar {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    border-top: var(--border-subtle);
    background: var(--bg-surface);
    z-index: 20;
    padding: 10px 32px;
  }

  .bottombar-inner {
    display: inline-flex;
    width: fit-content;
    align-items: center;
  }

  .add {
    width: 220px;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 80;
  }

  .modal {
    width: 520px;
    max-width: calc(100vw - 32px);
    background: var(--bg-surface);
    border: var(--border-subtle);
    padding: 16px;
    box-sizing: border-box;
  }

  .modal-title {
    font-family: var(--font-ui);
    color: var(--interactive);
    margin-bottom: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
    color: var(--text-muted);
    font-family: var(--font-ui);
  }

  .field input {
    width: 100%;
    background: var(--bg);
    color: var(--text);
    border: var(--border-subtle);
    border-radius: var(--border-radius);
    padding: 8px 10px;
    font-family: var(--font-ui);
  }

  .path-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    align-items: center;
  }

  .pick-btn {
    border: var(--border-subtle);
    background: var(--bg);
    color: var(--text);
    border-radius: var(--border-radius);
    padding: 8px 12px;
    font-family: var(--font-ui);
    cursor: pointer;
  }

  .pick-btn:hover {
    border: var(--border);
    color: var(--interactive);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .modal-btn {
    border: 0;
    border-radius: var(--border-radius);
    padding: 8px 12px;
    font-family: var(--font-ui);
    cursor: pointer;
  }

  .modal-btn.cancel {
    background: var(--ash);
    color: var(--text);
  }

  .modal-btn.add {
    background: var(--interactive);
    color: var(--btn-primary-text);
  }

  .modal-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
