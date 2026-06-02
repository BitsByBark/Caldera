<p align="center">
  <img src="caldera_logo.svg" alt="CALDERA" width="400">
</p>

<p align="center">
  <a href="https://github.com/BitsByBark/Caldera/actions/workflows/release.yml">
    <img src="https://github.com/BitsByBark/Caldera/actions/workflows/release.yml/badge.svg" alt="Build Status">
  </a>
  <a href="https://github.com/BitsByBark/Caldera/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-GPL--3.0-blue.svg" alt="License: GPL-3.0">
  </a>
  <img src="https://img.shields.io/badge/status-pre--alpha-orange.svg" alt="Pre-alpha">
</p>

> ⚠️ **pre-alpha. things will break, features are missing, builds may be incompatible with each other.**

# CALDERA

mod manager built out of frustration. the popular one doesnt run on linux. we have gone too long that is not acceptable.

linux-first. windows supported. no account, no paywall, no electron.

they not like us. and neither is this.

also if you wanna look into it, find out the meaning of the project out there.

---

## Why CALDERA

- **linux-first** - not a wine port or a "we're working on it". runs natively
- **no paywalls** - parallel downloads, collections, profiles, all free
- **deployer system** - per-game TOML configs that define exactly where mod files go. write your own without recompiling anything
- **registry-based tracking** - every deployed file is logged. enable/disable by rename, state survives crashes
- **REBELLION theme** - ships with a full theme out of the box, more coming
- **actually lightweight** - tauri + native webview, not a chromium wrapper

---

## Install

latest builds on the [Releases page](https://github.com/BitsByBark/Caldera/releases).

### Linux

**Fedora / RHEL / openSUSE (`.rpm`)**
```
sudo dnf install ./caldera-*.rpm
```

**Debian / Ubuntu (`.deb`)**
```
sudo apt install ./caldera_*.deb
```

**AppImage (anywhere)**
```
chmod +x CALDERA-*.AppImage
./CALDERA-*.AppImage
```

**Arch** - use the AppImage for now, AUR is on the list.

### Windows

grab the `.msi` or `.exe` from releases and run it.

---

## Whats in it right now (alpha)

### game library
- scans your steam library automatically (linux and windows paths)
- add non-steam games manually by pointing at the install folder
- caches steam artwork locally (banner, hero, logo)
- per-game config: mod folder, deployer, profiles

### profiles
- multiple profiles per game
- per-profile mod list with enable/disable
- profile mods tracked seperately from global installs

### deployers
- deployer system is TOML-defined, drop a file in `defaults/deployers/` and it gets picked up
- **unreal engine deployer** ships out of the box - handles `.pak/.utoc/.ucas` files, drops them in `~mods`, groups by basename, alphabetical load order
- select deployer per game

### mod operations
- extract `.zip` archives into staging
- deploy to game folder, files tracked in `registry.caldera`
- enable/disable without deleting anything (renames to `.disabled`, recovers state on next launch)
- full per-mod manifest with every file path

### settings
- `.brk` settings format with live reload
- theme, accent color, ui scale, text scale all hot-reloadable
- working directory is configurable, move the whole runtime wherever

### packaging
- push a tag, get `.rpm` `.deb` `.AppImage` `.msi` `.exe` out the other side via github actions

---

## whats blocking beta

stuff that needs to exist before this is usable for a real person:

- [ ] **mod install from zip** - extract, build manifest, move to storage
- [ ] **deploy** - copy files to game folder via deployer, write to registry
- [ ] **enable/disable** - rename `.ext` to `.ext.disabled` and back, keep manifest in sync
- [ ] **uninstall** - delete deployed files, remove from registry, keep storage
- [ ] **state recovery** - on launch scan registry for `.disabled` files and reconcile
- [ ] **profiles** - actually persist them, create/delete/rename, save modlist to `.profile` file
- [ ] **UE deployer tested end to end** on a real game
- [ ] **windows path handling** - separators, steam detection from registry, game-running guard
- [ ] **basic errors** - bad zip, file missing, path wrong, game is open

---

## roadmap past beta

no particular order, no dates.

### next up
- metadata and artwork actually showing (steam appcache pull)
- manual game add with exe path for launcher support later
- conflict detection
- dry run mode (logs what would happen, doesnt touch anything)
- restore points before deploys
- `.7z` `.rar` `.tar.*` support

### mid
- download pipeline (`parallel_downloads` is wired up in settings, actual downloading isnt built yet)
- browser extension for catching downloads (chrome + firefox)
- more deployers: bepinex, melonloader, bethesda data folders, RE engine, generic overlay
- FOMOD support
- profile sharing as `.caldera` files
- load order editor
- AUR package

### later
- nexus/gamebanana/thunderstore integrations
- LLM deployer suggestions for games we dont have a deployer for yet
- community deployer submissions
- proton/wine awareness
- plugin api
- theme editor

---

## building from source

```
# frontend
cd frontend && npm install && npm run build

# backend
cd ../backend
cargo install tauri-cli --version "^2.0" --locked
cargo tauri dev
```

### linux deps

**debian/ubuntu**
```
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev build-essential curl wget file
```

**fedora**
```
sudo dnf install webkit2gtk4.1-devel gtk3-devel libayatana-appindicator-gtk3-devel librsvg2-devel libsoup3-devel @development-tools curl wget file
```

**arch**
```
sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg libsoup3 base-devel curl wget file
```

---

## releasing

```
git tag v0.1-alpha
git push origin v0.1-alpha
```

actions picks it up and builds everything. tag with a `-` in it gets flagged as pre-release automatically.

if a build fails and you need to pull the tag:

```
git tag -d v0.1-alpha
git push origin :refs/tags/v0.1-alpha
```

---

## contributing

prs, issues, deployer TOMLs, themes, all welcome. for deployers just match the format in `defaults/deployers/` and use the UE deployer as reference.

discord: **[TODO]**

---

## license

GPL-3.0, see [LICENSE](LICENSE). fork it, ship it, just keep it open.
