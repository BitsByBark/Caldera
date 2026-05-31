<!-- UI Scale -->
<!--
<RulerSlider
  label="UI SCALE"
  stops={[
    { value: 75, label: '75%' },
    { value: 100, label: '100%' },
    { value: 125, label: '125%' },
    { value: 150, label: '150%' },
  ]}
  value={100}
  onChange={v => uiScale = v}
/>
-->

<!-- Text Scale -->
<!--
<RulerSlider
  label="TEXT SCALE"
  stops={[
    { value: 80, label: '80%' },
    { value: 90, label: '90%' },
    { value: 100, label: '100%' },
    { value: 110, label: '110%' },
    { value: 120, label: '120%' },
  ]}
  value={100}
  onChange={v => textScale = v}
/>
-->

<script>
  import { onMount } from 'svelte';

  export let stops = [];
  export let value;
  export let label = '';
  export let onChange = () => {};

  let rulerEl;
  let dragging = false;
  let dragX = 0;

  $: safeStops = Array.isArray(stops) ? stops : [];
  $: clampedDragX = Math.max(0, Math.min(1, dragX));
  $: activeIndex = Math.max(0, safeStops.findIndex((s) => s.value === value));
  $: activeStop = safeStops[activeIndex] || null;
  $: activePos = safeStops.length > 1 ? activeIndex / (safeStops.length - 1) : 0;
  $: thumbPos = dragging ? clampedDragX : activePos;
  $: activeDisplay = activeStop ? (activeStop.label === '100%' ? 'DEFAULT' : String(activeStop.label).toUpperCase()) : '';

  function eventClientX(event) {
    if (event.touches && event.touches.length) return event.touches[0].clientX;
    if (event.changedTouches && event.changedTouches.length) return event.changedTouches[0].clientX;
    return event.clientX;
  }

  function toRatio(event) {
    if (!rulerEl) return 0;
    const rect = rulerEl.getBoundingClientRect();
    const x = eventClientX(event) - rect.left;
    if (rect.width <= 0) return 0;
    return Math.max(0, Math.min(1, x / rect.width));
  }

  function nearestIndex(ratio) {
    if (!safeStops.length) return 0;
    if (safeStops.length === 1) return 0;
    const raw = ratio * (safeStops.length - 1);
    return Math.max(0, Math.min(safeStops.length - 1, Math.round(raw)));
  }

  function snapToRatio(ratio) {
    if (!safeStops.length) return;
    const idx = nearestIndex(ratio);
    const next = safeStops[idx];
    if (next && next.value !== value) onChange(next.value);
  }

  function onTrackClick(event) {
    if (dragging) return;
    snapToRatio(toRatio(event));
  }

  function startDrag(event) {
    event.preventDefault();
    dragging = true;
    dragX = toRatio(event);
  }

  function moveDrag(event) {
    if (!dragging) return;
    dragX = toRatio(event);
  }

  function endDrag(event) {
    if (!dragging) return;
    dragging = false;
    snapToRatio(toRatio(event));
  }

  onMount(() => {
    window.addEventListener('mousemove', moveDrag);
    window.addEventListener('mouseup', endDrag);
    window.addEventListener('touchmove', moveDrag, { passive: false });
    window.addEventListener('touchend', endDrag);

    return () => {
      window.removeEventListener('mousemove', moveDrag);
      window.removeEventListener('mouseup', endDrag);
      window.removeEventListener('touchmove', moveDrag);
      window.removeEventListener('touchend', endDrag);
    };
  });
</script>

<div class="ruler-slider">
  <div class="top-label">{`{ ${String(label).toUpperCase()} }`}</div>

  <div class="ruler" bind:this={rulerEl} on:click={onTrackClick}>
    {#if activeStop}
      <div class="active-caption" style={`left: ${thumbPos * 100}%`}>
        <span class="cap-tick"></span>
        <span class="cap-text">{activeDisplay}</span>
        <span class="cap-tick"></span>
      </div>
    {/if}

    <div class="track"></div>

    {#each safeStops as stop, i}
      <div class="tick" style={`left: ${(safeStops.length > 1 ? i / (safeStops.length - 1) : 0) * 100}%`}></div>
    {/each}

    <div
      class="thumb"
      style={`left: ${thumbPos * 100}%`}
      on:mousedown={startDrag}
      on:touchstart={startDrag}
    ></div>
  </div>

  <div class="tick-label-row">
    {#each safeStops as stop, i}
      <div class="tick-label" style={`left: ${(safeStops.length > 1 ? i / (safeStops.length - 1) : 0) * 100}%`}>{stop.label}</div>
    {/each}
  </div>
</div>

<style>
  .ruler-slider {
    width: 100%;
    background: transparent;
    font-family: var(--font);
    color: var(--text);
  }

  .top-label {
    text-transform: uppercase;
    margin-bottom: 14px;
  }

  .ruler {
    position: relative;
    height: 58px;
    cursor: pointer;
    user-select: none;
    -webkit-user-select: none;
    touch-action: none;
  }

  .track {
    position: absolute;
    left: 0;
    right: 0;
    top: 29px;
    border-top: 2px solid var(--slider-track);
  }

  .tick {
    position: absolute;
    width: 0;
    height: 24px;
    top: 17px;
    border-left: 1px solid var(--slider-tick);
    transform: translateX(-0.5px);
    pointer-events: none;
  }

  .thumb {
    position: absolute;
    width: 16px;
    height: 28px;
    top: 15px;
    transform: translateX(-8px);
    background: var(--slider-thumb);
    border-radius: var(--border-radius);
    box-shadow: none;
    cursor: pointer;
  }

  .active-caption {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    font-size: 12px;
    text-transform: uppercase;
    color: var(--text);
  }

  .cap-text {
    line-height: 1;
  }

  .cap-tick {
    width: 0;
    height: 12px;
    border-left: 1px solid var(--slider-tick);
    display: inline-block;
  }

  .tick-label-row {
    position: relative;
    height: 18px;
    margin-top: 6px;
  }

  .tick-label {
    position: absolute;
    transform: translateX(-50%);
    color: var(--slider-label);
    font-size: 11px;
    white-space: nowrap;
  }
</style>
