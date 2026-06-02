# CALDERA — CODEX CONTEXT

Cross-platform mod manager. Linux-first, Windows supported. Tauri 2 + Svelte + Rust. No Electron, no login, no paywalls.

---

## STACK
- Backend: Rust, Tauri 2, axum (`localhost:7337`)
- Frontend: Svelte + Vite, svelte-spa-router
- Config: TOML (per-game/profile), `.brk` (settings), `.caldera` (registry)

## REPO STRUCTURE
```
caldera/
  backend/            Rust, Tauri core, all logic
  frontend/           Svelte, all UI
  defaults/
    themes/           rebellion.css (active), base.css
    deployers/        deployer TOMLs (seeded to ~/.config/caldera/defaults/ on first run)
```

## FILE STORAGE
```
~/.config/caldera/
  settings.brk
  registry.caldera
  defaults/deployers/
  downloads/
  cache/{APPID}/
    meta.json         name, install path, size, last played, build id
    config.toml       deployer, active profile
    artwork/          banner.jpg, hero.jpg, logo.png
    mods/{MOD_ID}/
      meta.toml       name, version, source, install date
      manifest.json   deployed files + enabled state
    profiles/{NAME}.toml
```

## REGISTRY FORMAT (.caldera syntax)
```
game "1091500" {
    name   = "Cyberpunk 2077"
    path   = "/path/to/game"
    exe    = "/path/to/game.exe"
    source = "steam"
    added  = "2026-06-01"
}

mod "vortex_fix" {
    game_id   = "1091500"
    game_name = "Cyberpunk 2077"
    installed = "2026-06-01"
    files     = ["/path/to/file.dll"]
}
```

## DEPLOYMENT MODEL
- Deploy = copy files from storage to game folder, log paths in registry
- Disable = rename `file.ext` to `file.ext.disabled`
- Enable = rename back
- Uninstall = delete deployed files, remove from registry
- State recovery on launch: scan registry paths for `.disabled` files
- Block all file ops if game process is running

## DEPLOYERS
TOML-defined, stored in `defaults/deployers/`. Seeded to `~/.config/caldera/defaults/deployers/` on first run so users can add their own.

Current: `unreal_engine.toml` (handles `.pak/.utoc/.ucas`, drops into `~mods`, groups by basename)

Resolve content path at deploy time: walk install root one level deep looking for `{GameName}/Content/Paks`. Fall back to `deployer_mod_path` override in `config.toml`.

## MANIFEST FORMAT
```json
{
  "deployed": true,
  "deployer": "unreal_engine",
  "target_folder": "/path/to/~mods",
  "files": [
    { "name": "MyMod_P.pak", "target": "/full/path", "enabled": true }
  ],
  "deployed_at": "2026-06-01T00:00:00Z"
}
```

## .brk SYNTAX
```
group_id "LABEL" {
    entry_id {
        label   = "Human readable"
        type    = bool|cycle ["a","b"]|range 0..100 step 1|text|path|color|keybind
        default = value
        hot     = true
    }
}
```
Settings files: `frontend/public/settings/caldera.brk` (app), `~/.config/caldera/settings.brk` (user — API keys, steam path)

## REBELLION THEME TOKENS
```
--bg: #1A1614        --bg-surface: #221E1B    --bg-hover: #2A2520
--text: #F0EDE6      --text-muted: #9A938C    --text-disabled: #4A4440
--interactive: #E8B84B (yellow — touchable)
--action: #C0392B    --action-active: #A93226  (red — being touched)
--ash: #4A4440       --success: #6B9E3F        --warning: #C96A2A
--error: #C0392B     --info: #4A7FA5
--border-radius: 2px
--font: "Courier New", Courier, monospace
```

## UI RULES
- All text monospaced
- Section headers: `{ LABEL }`, active tabs: `>> LABEL`
- Expand/collapse arrows: `↓` / `↑`
- Borders: yellow (interactive), red (action), ash (subtle) — never bright white

## SVELTE STORES
`game.js` — gameList, currentGame
`profile.js` — profiles, activeProfile
`settings.js` — all settings values
`log.js` — sessionLog, addLog(message, type)

## ROUTES
`/` GameSelect, `/game/:id` GameDetail, `/settings` Settings

## TAURI COMMANDS
```rust
deploy_mod(app_id, mod_id) -> Result<ModManifest, String>
undeploy_mod(app_id, mod_id) -> Result<(), String>
toggle_mod(app_id, mod_id, enabled) -> Result<(), String>
resolve_deployer_path(app_id, deployer_id) -> Result<String, String>
```
Log to frontend via Tauri events, not command return values.

## BROWSER EXTENSION
MV3, Chrome + Firefox. Injects "⬇ CALDERA" button on Nexus mod pages next to manual download. On click:
1. Pulls `game_domain` + `mod_id` from page URL, `file_id` from download link params
2. POSTs to `localhost:7337/download` with url, game_domain, mod_id, file_id
3. Backend fetches full Nexus API metadata (mod details, files, changelogs), stores raw in `cache/{game_domain}/{mod_id}/meta.json`
4. Stubs download queue (real pipeline TBD)

Nexus API key stored in `~/.config/caldera/settings.brk` under `accounts`.

## CURRENT ROADMAP
```
ALPHA (now)
  [x] Cargo workspace, Tauri + Svelte init
  [x] REBELLION theme + UI kit
  [x] Steam library scan (VDF + .acf)
  [x] Game select + game detail view
  [ ] Mod install from zip
  [ ] Deploy / enable / disable / uninstall
  [ ] State recovery on launch
  [ ] Error handling

DEPLOYERS
  [ ] UE deployer end-to-end on real game
  [ ] Cyberpunk deployer

WINDOWS COMPAT
  [ ] Windows Steam path (registry scan)
  [ ] Path separator handling (std::path::Path throughout)
  [ ] Game-running guard (stub: is_game_running() returns false + TODO)

BETA
  [ ] Real profile persistence
  [ ] Manual game add
  [ ] Metadata + Steam artwork display
  [ ] Conflict detection
  [ ] Dry run mode + restore points

V1
  [ ] Download pipeline
  [ ] Browser extension shipped
  [ ] FOMOD support
  [ ] Profile sharing (.caldera files)
  [ ] AUR + Windows installer
  [ ] Plugin API

POST-LAUNCH
  [ ] LLM deployer suggestions (Claude API, user key)
  [ ] Community deployer database
  [ ] Game launcher integration
  [ ] Theme editor
```

## RELEASE NOTES STYLE
Written in the GitHub release body when a tag is pushed. Same tone as the readme — lowercase, no fluff, no AI voice.

Structure:
```
## whats new in {version}

### added
- thing that was added
- another thing

### fixed
- bug that was fixed

### changed
- behaviour that changed

### removed
- thing that was removed
```

Rules:
- only include sections that have entries, skip empty ones
- one line per change, no waffle
- if it's a pre-alpha/alpha/beta tag add a one-liner at the top like "pre-alpha. expect breakage."
- no "thank you to contributors" section until there are actual contributors
- no em dashes, no marketing language
- match the commit messages — if commits say `ADDED - ue deployer` then release notes say `- ue deployer`

---

## README STYLE
When updating README.md follow these rules exactly:

- lowercase headers and body text throughout
- no em dashes, no tricolon lists, no parallel bullet structure
- no AI-sounding closers ("just works", "that's the deal", "seamlessly")
- bullets are functional, not sales-y — say what it does, not why it's amazing
- short paragraphs, no waffle
- intentional typos are fine ("doesnt", "seperately") — don't correct them
- kendrick reference stays in the hook, don't touch it
- roadmap stays as flat checklists, no dates, no commitments
- keep the pre-alpha warning at the top
- don't add screenshots until told to
- when features ship, move them from roadmap to current features section
- when the deployment model, file structure, or roadmap changes — update the readme to match

---

## COMMIT STYLE
Format: `PREFIX - description, maybe description 2`
All caps prefix, lowercase description, comma-separate multiple changes.

Commit after each completed feature or verified fix. Do not push unless explicitly told to push. Do not commit partial one-file steps unless they complete a feature or fix.

Prefixes:
- `ADDED` - new feature or file
- `FIXED` - bug fix
- `CHANGED` - behaviour or logic change
- `REMOVED` - deleted feature or file
- `REFACTOR` - restructure with no behaviour change
- `CHORE` - deps, config, tooling, cleanup
- `WIP` - incomplete, not ready

Examples:
```
ADDED - ue deployer, manifest write on deploy
FIXED - windows path separator in registry scan
CHANGED - deploy model from symlink to copy+rename
REMOVED - symlink logic from mod core
CHORE - add axum dep, update Cargo.lock
WIP - download pipeline stub, not wired yet
```

---

## RULES
- ask before structural decisions
- one step at a time, confirm before continuing
- use std::path::Path for all paths, no hardcoded separators
- Tauri events for logs, not return values
- copy files, no symlinks
- never delete a file not in the manifest
- if anything in this doc contradicts what you see in the actual codebase, stop and ask — then suggest an update to this doc
