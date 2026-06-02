<script>
  import Router from 'svelte-spa-router';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getSettingsValues, saveSettingsValues, setWorkingDirectory } from './lib/tauri.js';
  import GameSelect from './routes/GameSelect.svelte';
  import GameDetail from './routes/GameDetail.svelte';
  import ProfilePage from './routes/ProfilePage.svelte';
  import Settings from './routes/Settings.svelte';
  import Toast from './components/Toast.svelte';
  import { settings } from './stores/settings.js';
  import { addLog, upsertProgressLog } from './stores/log.js';
  let settingsHydrated = false;
  let saveTimer = null;

  const routes = {
    '/': GameSelect,
    '/game/:id': GameDetail,
    '/game/:id/profile': ProfilePage,
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
    setWorkingDirectory(s.working_directory || null).catch(() => {});

    if (!settingsHydrated) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveSettingsValues(s).catch(() => {});
    }, 150);
  });

  onMount(() => {
    let unlistenSession;
    let unlistenOp;
    let unlistenDownload;

    (async () => {
      try {
        const persisted = await getSettingsValues();
        if (persisted && typeof persisted === 'object') {
          settings.update((current) => ({ ...current, ...persisted }));
        }
      } catch (_e) {
        // Ignore settings load failures and use store defaults.
      } finally {
        settingsHydrated = true;
      }

      try {
        unlistenSession = await listen('caldera://session-log', (event) => {
          const payload = event.payload || {};
          if (!payload.message) return;
          addLog(payload.message, payload.level || 'info');
        });
        unlistenOp = await listen('caldera://operation-progress', (event) => {
          const payload = event.payload || {};
          if (payload.operation !== 'uncompress') return;
          const key = `${payload.operation}:${payload.target || 'unknown'}`;
          upsertProgressLog(key, payload.message || 'Uncompressing...', Number(payload.progress || 0), 'info');
        });
        unlistenDownload = await listen('caldera://download-progress', (event) => {
          const payload = event.payload || {};
          const key = payload.key || `download:${payload.game_domain || 'unknown'}:${payload.mod_id || 'unknown'}:${payload.file_id || 'unknown'}`;
          upsertProgressLog(key, payload.message || 'Downloading...', Number(payload.progress || 0), 'info');
        });
      } catch (_e) {
        // Ignore event listener failures in non-tauri contexts.
      }
    })();

    return () => {
      if (saveTimer) clearTimeout(saveTimer);
      if (unlistenSession) unlistenSession();
      if (unlistenOp) unlistenOp();
      if (unlistenDownload) unlistenDownload();
    };
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
