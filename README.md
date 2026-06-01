<p align="center">
  <img src="caldera_logo.svg" alt="CALDERA" width="160" />
</p>

# CALDERA

Cross-platform mod manager. Linux-first, Windows supported.

Built with Tauri 2 (Rust backend, Svelte frontend). Aims to be a Vortex-class mod manager that treats Linux as a primary target — not an afterthought.

---

## Install

Grab the latest build from the [Releases page](../../releases).

### Linux

**Fedora / RHEL / openSUSE (`.rpm`)**
```bash
sudo dnf install ./caldera-*.rpm
```

**Debian / Ubuntu (`.deb`)**
```bash
sudo apt install ./caldera_*.deb
```

**Anywhere (`.AppImage`)**
```bash
chmod +x CALDERA-*.AppImage
./CALDERA-*.AppImage
```

**Arch** — use the AppImage for now (native AUR package planned).

### Windows

Download the `.msi` or `.exe` installer and double-click it.

---

## Current Features

### Game library
- Auto-detects installed Steam games (Linux + Windows Steam paths)
- Manually add non-Steam games by install path
- Pulls Steam library artwork (banner, hero, logo) into a local cache
- Per-game configuration: mod directory, active deployer, profiles

### Profiles
- Multiple named profiles per game
- Profile-scoped mod list with enable/disable toggles
- Profile mod rows tracked separately from globally-installed mods

### Deployers
- Pluggable deployer system (TOML-defined) that places mod files into a game's mod directory
- **Unreal Engine deployer** included: handles `.pak` / `.utoc` / `.ucas` files via the `~mods` folder convention, with basename grouping and alphabetical load order
- Per-game deployer selection — pick the right one for each game

### Mod operations
- Uncompress mod archives (`.zip`) into staging
- Deploy / undeploy individual mods to the game directory
- Enable / disable mods without removing them
- Listings → deployed mod manifest tracking

### Settings
- Custom `.brk` settings schema format with live parsing on the frontend
- Hot-reloadable settings (theme, accent color, scaling, sort order, parallel downloads…)
- Configurable working directory — relocate the entire CALDERA runtime (cache, downloads, metadata) anywhere on disk
- Per-game default-game preference

### Appearance
- Theme system (ships with `rebellion`)
- Custom accent color
- UI scale + text scale independently adjustable
- Native directory chooser dialogs

### Packaging
- Tagged commits auto-build `.rpm`, `.deb`, `.AppImage`, `.msi`, and `.exe` via GitHub Actions and publish to a Release

---

## Roadmap

Rough order, not commitments.

### Near-term
- [ ] Mod downloads (the `parallel_downloads` setting is wired but the download pipeline is still being built)
- [ ] Metadata cache (global / per-game / auto modes — schema is defined)
- [ ] Sort orders: name / date / size / status (UI hookup)
- [ ] Native Arch / AUR package
- [ ] Additional archive formats (`.7z`, `.rar`, `.tar.*`)

### Mid-term
- [ ] More deployers (BepInEx, MelonLoader, Skyrim/Bethesda data dirs, generic file overlay)
- [ ] Mod source integrations (Nexus, GameBanana, Thunderstore)
- [ ] Profile import/export & sharing
- [ ] Load-order editor for deployers that need it
- [ ] Dependency / conflict resolution

### Longer-term
- [ ] Wine / Proton awareness for Windows-only games on Linux
- [ ] Cloud profile sync
- [ ] Additional themes & a theme editor
- [ ] Plugin API for community deployers and sources

---

## Building from source

```bash
# Frontend
cd frontend && npm install && npm run build

# Tauri app (run from backend/)
cd ../backend
cargo install tauri-cli --version "^2.0" --locked
cargo tauri build      # or `cargo tauri dev` for hot-reload
```

Linux build deps (Debian/Ubuntu names):
`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev`

---

## Releasing

To trigger a release build:

```bash
git tag v0.1-alpha
git push origin v0.1-alpha
```

GitHub Actions will build Linux + Windows packages and publish them automatically.

Pre-release flag is auto-set if the tag contains a `-` (e.g. `v0.1-alpha`, `v0.4-beta`).

To delete a tag if a build fails:

```bash
git tag -d v0.1-alpha
git push origin :refs/tags/v0.1-alpha
```
