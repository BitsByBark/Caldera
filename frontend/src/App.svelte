<script>
  import Router from 'svelte-spa-router';
  import GameSelect from './routes/GameSelect.svelte';
  import GameDetail from './routes/GameDetail.svelte';
  import Settings from './routes/Settings.svelte';
  import Toast from './components/Toast.svelte';
  import { settings } from './stores/settings.js';

  const routes = {
    '/': GameSelect,
    '/game/:id': GameDetail,
    '/settings': Settings
  };

  async function loadTheme(themeName) {
    const existing = document.getElementById('caldera-theme');
    if (existing) existing.remove();

    const res = await fetch(`/themes/${themeName}.css`);
    const css = await res.text();
    const style = document.createElement('style');
    style.id = 'caldera-theme';
    style.textContent = css;
    document.head.appendChild(style);
  }

  settings.subscribe((s) => {
    loadTheme(s.theme ?? 'rebellion');
    document.documentElement.style.setProperty('--ui-scale', s.ui_scale / 100);
    document.documentElement.style.setProperty('--text-scale', s.text_scale / 100);
    document.documentElement.style.setProperty('--interactive', s.accent_color ?? '#E8B84B');
  });
</script>

<main class="app">
  <Router {routes} />
  <Toast />
</main>

<style>
  :global(html, body, #app) {
    margin: 0;
    min-height: 100%;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font);
  }

  :global(body) {
    font-size: calc(14px * var(--text-scale));
  }

  .app {
    min-height: 100vh;
    zoom: calc(var(--ui-scale));
  }
</style>
