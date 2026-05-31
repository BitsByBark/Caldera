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

function stripComments(text) {
  return text
    .split('\n')
    .map((line) => line.replace(/\/\/.*$/, ''))
    .join('\n');
}

function skipWhitespace(source, idx) {
  let i = idx;
  while (i < source.length && /\s/.test(source[i])) i += 1;
  return i;
}

function readQuoted(source, idx) {
  let i = skipWhitespace(source, idx);
  if (source[i] !== '"') throw new Error(`Expected quoted string at index ${i}`);
  i += 1;
  const start = i;
  while (i < source.length && source[i] !== '"') i += 1;
  if (i >= source.length) throw new Error('Unterminated quoted string');
  return { value: source.slice(start, i), next: i + 1 };
}

function readIdentifier(source, idx) {
  let i = skipWhitespace(source, idx);
  const start = i;
  while (i < source.length && /[a-z0-9_]/i.test(source[i])) i += 1;
  if (i === start) throw new Error(`Expected identifier at index ${i}`);
  return { value: source.slice(start, i), next: i };
}

function extractBlock(source, openBraceIdx) {
  let depth = 0;
  let i = openBraceIdx;
  for (; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    else if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) {
        return { content: source.slice(openBraceIdx + 1, i), next: i + 1 };
      }
    }
  }
  throw new Error('Unterminated block');
}

function parseFields(entryBody, entryId) {
  const fieldRe = /^\s*([a-z0-9_]+)\s*=\s*(.+)$/gm;
  const fields = {};
  let fieldMatch;

  while ((fieldMatch = fieldRe.exec(entryBody)) !== null) {
    fields[fieldMatch[1]] = fieldMatch[2].trim();
  }

  const required = ['label', 'type', 'default', 'hot'];
  for (const req of required) {
    if (!(req in fields)) throw new Error(`Missing required field "${req}" in entry ${entryId}`);
  }

  return {
    id: entryId,
    label: parseValue(fields.label),
    type: parseType(fields.type),
    default: parseValue(fields.default),
    hot: parseValue(fields.hot),
    desc: fields.desc ? parseValue(fields.desc) : null,
  };
}

function parseEntries(groupBody) {
  const entries = [];
  let idx = 0;

  while (idx < groupBody.length) {
    idx = skipWhitespace(groupBody, idx);
    if (idx >= groupBody.length) break;

    const { value: entryId, next: afterId } = readIdentifier(groupBody, idx);
    let cursor = skipWhitespace(groupBody, afterId);
    if (groupBody[cursor] !== '{') {
      throw new Error(`Expected "{" after entry ${entryId}`);
    }

    const { content: entryBody, next } = extractBlock(groupBody, cursor);
    entries.push(parseFields(entryBody, entryId));
    idx = next;
  }

  return entries;
}

export function parseBrk(text) {
  const source = stripComments(text);
  const groups = [];
  let idx = 0;

  while (idx < source.length) {
    idx = skipWhitespace(source, idx);
    if (idx >= source.length) break;

    const { value: groupId, next: afterGroupId } = readIdentifier(source, idx);
    const { value: groupLabel, next: afterLabel } = readQuoted(source, afterGroupId);
    const cursor = skipWhitespace(source, afterLabel);
    if (source[cursor] !== '{') {
      throw new Error(`Expected "{" after group ${groupId}`);
    }

    const { content: groupBody, next } = extractBlock(source, cursor);
    groups.push({ id: groupId, label: groupLabel, entries: parseEntries(groupBody) });
    idx = next;
  }

  return { groups };
}
