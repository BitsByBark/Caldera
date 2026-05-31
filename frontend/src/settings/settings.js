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
    return {
      kind: 'range',
      min: Number(rangeMatch[1]),
      max: Number(rangeMatch[2]),
      step: Number(rangeMatch[3]),
    };
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
