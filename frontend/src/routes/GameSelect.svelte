<script>
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { push } from 'svelte-spa-router';
  import Button from '../components/Button.svelte';
  import TopBar from '../components/TopBar.svelte';
  import { gameList, loadGameList, currentGame } from '../stores/game';
  import { getGameArtwork } from '../lib/tauri';

  let artworkById = {};
  let posterLoadFailed = {};

  onMount(async () => {
    await loadGameList();
    const list = get(gameList);
    for (const game of list) {
      artworkById[game.app_id] = await getGameArtwork(game.app_id);
    }
  });

  function openGame(game) {
    currentGame.set(game);
    push(`/game/${game.app_id}`);
  }

  function markPosterFailed(appId) {
    posterLoadFailed = { ...posterLoadFailed, [appId]: true };
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
      <div class="add"><Button variant="secondary" icon="+" label="ADD GAME" /></div>
    </div>
  </div>
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
</style>
