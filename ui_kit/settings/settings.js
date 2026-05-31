import { initInputs } from '../inputs/inputs.js';
import { initLists } from '../lists/lists.js';

function parseType(typeRaw) {
  const t = typeRaw.trim();
  if (t === 'bool' || t === 'select' || t === 'text' || t === 'path' || t === 'color' || t === 'keybind') {
    return { kind: t };
  }

  const cycleMatch = t.match(/^cycle\s*\[(.*)\]$/);
  if (cycleMatch) {
    const options = cycleMatch[1]
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean)
      .map((s) => s.replace(/^"|"$/g, ''));
    return { kind: 'cycle', options };
  }

  const rangeMatch = t.match(/^range\s+(-?\d+(?:\.\d+)?)\.\.(-?\d+(?:\.\d+)?)\s+step\s+(-?\d+(?:\.\d+)?)$/);
  if (rangeMatch) {
    const min = Number(rangeMatch[1]);
    const max = Number(rangeMatch[2]);
    const step = Number(rangeMatch[3]);
    return { kind: 'range', min, max, step };
  }

  throw new Error(`Unknown type expression: ${t}`);
}

function parseValue(raw) {
  const v = raw.trim();
  if (/^".*"$/.test(v)) return v.slice(1, -1);
  if (v === 'true') return true;
  if (v === 'false') return false;
  if (/^-?\d+(?:\.\d+)?$/.test(v)) return Number(v);
  return v;
}

export function parseBrk(text) {
  const stripped = text
    .split('\n')
    .map((line) => line.replace(/\/\/.*$/, '').trimEnd())
    .join('\n');

  const groupRe = /([a-z0-9_]+)\s+"([^"]+)"\s*\{([\s\S]*?)\}/g;
  const entryRe = /([a-z0-9_]+)\s*\{([\s\S]*?)\}/g;
  const fieldRe = /^\s*([a-z0-9_]+)\s*=\s*(.+)$/gm;

  const groups = [];
  let groupMatch;
  while ((groupMatch = groupRe.exec(stripped)) !== null) {
    const groupId = groupMatch[1];
    const groupLabel = groupMatch[2];
    const groupBody = groupMatch[3];

    const entries = [];
    let entryMatch;
    while ((entryMatch = entryRe.exec(groupBody)) !== null) {
      const entryId = entryMatch[1];
      const entryBody = entryMatch[2];
      const fields = {};
      let fieldMatch;
      while ((fieldMatch = fieldRe.exec(entryBody)) !== null) {
        fields[fieldMatch[1]] = fieldMatch[2].trim();
      }

      const required = ['label', 'type', 'default', 'hot'];
      for (const req of required) {
        if (!(req in fields)) throw new Error(`Missing required field \"${req}\" in entry ${entryId}`);
      }

      entries.push({
        id: entryId,
        label: parseValue(fields.label),
        type: parseType(fields.type),
        default: parseValue(fields.default),
        hot: parseValue(fields.hot),
        desc: fields.desc ? parseValue(fields.desc) : null,
      });
    }

    groups.push({ id: groupId, label: groupLabel, entries });
  }

  return { groups };
}

function inputForEntry(entry) {
  const wrap = document.createElement('div');

  if (entry.type.kind === 'bool') {
    wrap.innerHTML = `
      <div class="toggle ${entry.default ? 'on' : ''}" data-toggle>
        <div class="toggle-pill"></div>
      </div>
    `;
    return wrap.firstElementChild;
  }

  if (entry.type.kind === 'text') {
    wrap.innerHTML = `<div class="input-shell"><input class="input-text" value="${entry.default}" placeholder="\"display name\"" /></div>`;
    return wrap.firstElementChild;
  }

  if (entry.type.kind === 'path') {
    wrap.innerHTML = `
      <div class="input-shell">
        <input class="input-path-field" value="${entry.default}" placeholder="\"~/path\"" />
        <button type="button" class="input-path-btn">...</button>
      </div>
    `;
    return wrap.firstElementChild;
  }

  if (entry.type.kind === 'select') {
    wrap.innerHTML = `
      <div class="dropdown" data-dropdown>
        <div class="input-shell dropdown-button" data-dropdown-button>
          <span>v</span><span data-dropdown-label>"${entry.default}"</span><span>v</span>
        </div>
        <ul class="dropdown-list">
          <li class="dropdown-item" data-dropdown-item>none</li>
          <li class="dropdown-item" data-dropdown-item>skyrim</li>
          <li class="dropdown-item" data-dropdown-item>fallout4</li>
        </ul>
      </div>
    `;
    return wrap.firstElementChild;
  }

  if (entry.type.kind === 'cycle') {
    wrap.innerHTML = `
      <div class="input-shell cycle" data-cycle data-options="${entry.type.options.join(',')}" data-value="${entry.default}">
        <button class="cycle-arrow" data-cycle-prev><</button>
        <span data-cycle-label></span>
        <button class="cycle-arrow" data-cycle-next>></button>
      </div>
    `;
    return wrap.firstElementChild;
  }

  if (entry.type.kind === 'range') {
    wrap.innerHTML = `
      <div class="input-shell range-wrap" data-range>
        <input type="range" min="${entry.type.min}" max="${entry.type.max}" step="${entry.type.step}" value="${entry.default}" />
        <span class="range-value" data-range-value>${entry.default}</span>
      </div>
    `;
    return wrap.firstElementChild;
  }

  if (entry.type.kind === 'color') {
    const color = String(entry.default).toUpperCase();
    wrap.innerHTML = `
      <div class="input-shell color-input" data-color-input>
        <div class="color-swatch" data-color-swatch style="background: ${color};"></div>
        <div data-color-value>${color}</div>
        <input type="color" value="${entry.default}" hidden />
      </div>
    `;
    return wrap.firstElementChild;
  }

  if (entry.type.kind === 'keybind') {
    wrap.innerHTML = `
      <div class="input-shell keybind-capture" tabindex="0" data-keybind>
        <span data-keybind-value>${entry.default}</span>
      </div>
    `;
    return wrap.firstElementChild;
  }

  return document.createTextNode('');
}

export function renderSettings(container, parsed) {
  container.innerHTML = '';
  for (const group of parsed.groups) {
    const section = document.createElement('section');
    section.className = 'section';

    const title = document.createElement('div');
    title.className = 'section-title';
    title.textContent = `{ ${group.label} }`;
    section.appendChild(title);

    const stack = document.createElement('div');
    stack.className = 'stack';

    for (const entry of group.entries) {
      const item = document.createElement('div');
      item.className = 'list-item';
      if (entry.desc) item.setAttribute('data-expandable', '');

      const main = document.createElement('div');
      main.className = 'list-main';

      const left = document.createElement('div');
      left.className = 'list-left';
      left.textContent = `"${entry.label}"`;

      const right = document.createElement('div');
      right.className = 'list-right';
      right.appendChild(inputForEntry(entry));

      const expand = document.createElement('div');
      expand.className = 'list-expand';
      if (entry.desc) {
        expand.setAttribute('data-expand-trigger', '');
        expand.innerHTML = '<span data-expand-icon>v</span>';
      }

      main.append(left, right, expand);
      item.appendChild(main);

      if (entry.desc) {
        const details = document.createElement('div');
        details.className = 'list-details settings-desc';
        details.setAttribute('data-expand-content', '');
        details.textContent = entry.desc;
        item.appendChild(details);
      }

      stack.appendChild(item);
    }

    section.appendChild(stack);
    container.appendChild(section);
  }

  initInputs(container);
  initLists(container);
}

export async function loadSettingsPage({ brkPath, mountSelector }) {
  const mount = document.querySelector(mountSelector);
  const text = await fetch(brkPath).then((r) => r.text());
  const parsed = parseBrk(text);
  renderSettings(mount, parsed);
}
