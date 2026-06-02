<script>
  import { onMount } from 'svelte';
  import { pop } from 'svelte-spa-router';
  import { get } from 'svelte/store';
  import TopBar from '../components/TopBar.svelte';
  import Button from '../components/Button.svelte';
  import SettingsListItem from '../components/SettingsListItem.svelte';
  import RulerSlider from '../components/RulerSlider.svelte';
  import { parseBrk } from '../settings/settings.js';
  import { settings } from '../stores/settings.js';
  import { gameList } from '../stores/game.js';
  import { getSettingsSchema, getSteamGames, setWorkingDirectory } from '../lib/tauri.js';
  import { showToast } from '../components/Toast.svelte';

  let groups = [];
  let activeGroupId = '';

  const uiScaleStops = [
    { value: 75, label: '75%' },
    { value: 100, label: '100%' },
    { value: 125, label: '125%' },
    { value: 150, label: '150%' },
  ];

  const textScaleStops = [
    { value: 80, label: '80%' },
    { value: 90, label: '90%' },
    { value: 100, label: '100%' },
    { value: 110, label: '110%' },
    { value: 120, label: '120%' },
  ];

  onMount(async () => {
    try {
      const text = await getSettingsSchema();
      const parsed = parseBrk(text);
      groups = parsed.groups;
      activeGroupId = parsed.groups[0]?.id || '';
    } catch (err) {
      showToast(`Failed to load settings schema: ${String(err)}`, 'error');
    }

    try {
      const wd = get(settings).working_directory || '';
      await setWorkingDirectory(wd || null);
    } catch (err) {
      showToast(`Working directory apply failed: ${String(err)}`, 'error');
    }
  });

  async function scanSteam() {
    const steamPath = get(settings).steam_path || null;
    try {
      const games = await getSteamGames(steamPath);
      gameList.set(games);
      showToast(`Found ${games.length} games`, 'success');
      if (games.length === 0) {
        showToast('No games found in Steam library.', 'warning');
      }
    } catch (err) {
      showToast(String(err), 'error');
    }
  }

  function updateSetting(groupId, key, value) {
    if (groupId === 'accounts') {
      settings.update((s) => ({
        ...s,
        accounts: {
          ...(s.accounts || {}),
          [key]: value,
        },
      }));
      return;
    }

    settings.update((s) => ({ ...s, [key]: value }));
    if (key === 'working_directory') {
      setWorkingDirectory(value || null).catch((err) => {
        showToast(`Working directory apply failed: ${String(err)}`, 'error');
      });
    }
  }

  $: activeGroup = groups.find((g) => g.id === activeGroupId) || groups[0] || null;
</script>

<section class="settings-page">
  <TopBar settingsActive={true} />

  <div class="settings-body">
    <aside class="settings-sidebar">
      <div class="sidebar-title">{'{ SETTINGS }'}</div>
      <div class="sidebar-rule"></div>

      <div class="sidebar-sections">
        {#each groups as group}
          <button
            class={`section-item ${activeGroupId === group.id ? 'active' : ''}`}
            on:click={() => (activeGroupId = group.id)}
          >
            {#if activeGroupId === group.id}&gt;&gt; {/if}{group.label.toUpperCase()}
          </button>
        {/each}
      </div>

      <div class="sidebar-spacer"></div>

      <div class="back-row">
        <Button variant="secondary" label="< BACK" onClick={() => pop()} />
      </div>
    </aside>

    <main class="settings-main">
      {#if activeGroup}
        <section class="group">
          <div class="group-title">{`{ ${activeGroup.label.toUpperCase()} }`}</div>
          <div class="entries">
            {#each activeGroup.entries as entry}
              <div class="entry-row">
                {#if entry.id === 'ui_scale'}
                  <div class="ruler-wrap">
                    <RulerSlider
                      label={entry.label}
                      stops={uiScaleStops}
                      value={$settings.ui_scale}
                      onChange={(v) => updateSetting(activeGroup.id, entry.id, v)}
                    />
                    {#if entry.desc}
                      <div class="entry-desc">{entry.desc}</div>
                    {/if}
                  </div>
                {:else if entry.id === 'text_scale'}
                  <div class="ruler-wrap">
                    <RulerSlider
                      label={entry.label}
                      stops={textScaleStops}
                      value={$settings.text_scale}
                      onChange={(v) => updateSetting(activeGroup.id, entry.id, v)}
                    />
                    {#if entry.desc}
                      <div class="entry-desc">{entry.desc}</div>
                    {/if}
                  </div>
                {:else}
                  <SettingsListItem
                    entry={entry}
                    value={activeGroup.id === 'accounts' ? $settings.accounts?.[entry.id] ?? entry.default : $settings[entry.id] ?? entry.default}
                    onChange={(v) => updateSetting(activeGroup.id, entry.id, v)}
                  />

                  {#if entry.id === 'steam_path'}
                    <div class="scan-row">
                      <Button variant="primary" icon="+" label="SCAN" onClick={scanSteam} />
                    </div>
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </main>
  </div>
</section>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font);
  }

  .settings-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    margin-top: 56px;
  }

  .settings-sidebar {
    display: flex;
    flex-direction: column;
    min-width: fit-content;
    height: 100%;
    background: var(--bg-surface);
    border-right: var(--border-subtle);
    padding: 16px 0;
  }

  .sidebar-title {
    padding: 8px 16px;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .sidebar-rule {
    border-bottom: var(--border-subtle);
    margin: 4px 0 8px;
  }

  .sidebar-sections {
    display: flex;
    flex-direction: column;
  }

  .section-item {
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font-family: var(--font);
    text-transform: uppercase;
    text-align: left;
    padding: 8px 16px;
    cursor: pointer;
    border-radius: var(--border-radius);
    box-shadow: none;
  }

  .section-item:hover {
    color: var(--action);
  }

  .section-item.active {
    color: var(--interactive);
  }

  .sidebar-spacer {
    flex: 1;
  }

  .back-row {
    padding: 0 16px;
  }

  .settings-main {
    flex: 1;
    overflow-y: auto;
    padding: 32px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-title {
    width: 100%;
    border-bottom: var(--border-subtle);
    padding-bottom: 8px;
    margin-bottom: 16px;
    text-transform: uppercase;
    color: var(--text);
  }

  .entries {
    display: flex;
    flex-direction: column;
  }

  .entry-row {
    border-bottom: var(--border-subtle);
    padding: 8px 0;
  }

  .ruler-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .entry-desc {
    color: var(--text-muted);
    font-size: 12px;
  }

  .scan-row {
    margin-top: 8px;
    width: 180px;
  }
</style>
