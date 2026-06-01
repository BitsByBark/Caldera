#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

test -f backend/icons/512x512.png
 test -f backend/icons/icon.png
 test -f backend/icons/icon.ico

python - << 'PY'
import struct
with open('backend/icons/512x512.png','rb') as f:
    sig=f.read(8)
    if sig!=b"\x89PNG\r\n\x1a\n":
        raise SystemExit('backend/icons/512x512.png is not PNG')
    clen=struct.unpack('>I',f.read(4))[0]
    ctyp=f.read(4)
    if ctyp!=b'IHDR' or clen<8:
        raise SystemExit('backend/icons/512x512.png missing IHDR')
    w,h=struct.unpack('>II',f.read(8))
if w!=h or w<256:
    raise SystemExit(f'Invalid icon size {w}x{h}; must be square and >=256')
print(f'Icon set OK: {w}x{h}')
PY
