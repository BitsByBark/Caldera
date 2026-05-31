<script>
  import { derived } from 'svelte/store';
  import TabBar from '../components/TabBar.svelte';
  import { gameList } from '../stores/game';

  export let params = {};
  const tabs = [
    { id: 'mods', label: 'MODS' },
    { id: 'collections', label: 'COLLECTIONS' },
    { id: 'profiles', label: 'PROFILES' }
  ];
  let activeTab = 'mods';

  const gameName = derived(gameList, ($games) => {
    const g = $games.find((x) => x.app_id === params.id);
    return g ? g.name : `Game ${params.id}`;
  });
</script>

<section class="page">
  <div class="top">
    <div>"{$gameName}"</div>
    <div class="mode">{'{ BROWSE MODE }'}</div>
  </div>

  <TabBar tabs={tabs} active={activeTab} orientation="horizontal" onChange={(id) => (activeTab = id)} />

  <div class="panel">
    {#if activeTab === 'mods'}"mods view placeholder"{/if}
    {#if activeTab === 'collections'}"collections view placeholder"{/if}
    {#if activeTab === 'profiles'}"profiles view placeholder"{/if}
  </div>
</section>

<style>
  .page { padding: 16px; display: flex; flex-direction: column; gap: 12px; }
  .top { display: flex; justify-content: space-between; align-items: center; gap: 8px; color: var(--text); }
  .mode { color: var(--text-muted); }
  .panel { background: var(--bg-surface); border: 1px solid transparent; border-radius: var(--border-radius); padding: 10px; min-height: 200px; color: var(--text-muted); }
</style>
