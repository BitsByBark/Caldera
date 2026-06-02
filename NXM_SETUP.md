# NXM Setup

CALDERA supports `nxm://` links through Tauri deep-link handling. Packaged desktop builds should register the scheme through the Tauri bundle metadata. Dev builds may need manual Linux registration.

## Linux Dev Registration

Create `~/.local/share/applications/caldera.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=CALDERA
Exec=/home/bark/MOUNTS/PROJECTS/CODE PLAYGROUND/caldera/backend/target/debug/caldera-backend %u
MimeType=x-scheme-handler/nxm;
NoDisplay=true
```

Register it:

```sh
xdg-mime default caldera.desktop x-scheme-handler/nxm
update-desktop-database ~/.local/share/applications
```

Verify:

```sh
xdg-mime query default x-scheme-handler/nxm
```

Expected:

```text
caldera.desktop
```

## Test Link

With CALDERA running, trigger an `nxm://` link from Nexus Mods using the Mod Manager Download button, or test manually:

```sh
xdg-open 'nxm://cyberpunk2077/mods/1234/files/5678?key=abc123&expires=1730000000&user_id=99999'
```

CALDERA should log `Received NXM link: ...` and then queue the download metadata.
