# Rift - Optimize. Launch. Enjoy.

All-in-one Windows game launcher, optimizer and Minecraft (Fabric) mod manager.
Tauri 2 + React/TypeScript frontend, Rust backend, local JSON storage. No account, no login.

## Status - please read

This source was written without being able to compile or run it (the build sandbox had no
network access and is not Windows). Syntax was checked, nothing else. **Expect a handful of compile
errors on first build** - paste them back and they are quick to fix. Features that touch Windows,
Steam, Epic, the Minecraft Launcher and Modrinth have never been run against the real thing.

## Build (Windows 10/11)

Prerequisites: [Node.js 20+](https://nodejs.org), [Rust](https://rustup.rs) (MSVC toolchain),
Visual Studio Build Tools with "Desktop development with C++", and WebView2 (preinstalled on Win 10/11).

```powershell
npm install
npm run tauri dev       # run with hot reload
npm run tauri build     # installer in src-tauri/target/release/bundle/nsis/
```

## What is real

| Area | What it does |
|---|---|
| Detection | Steam (libraryfolders.vdf + appmanifests), Epic manifests, Minecraft Java/Bedrock, Roblox, VALORANT (Riot Client), plus folders/drives you choose. Drive scans only read well-known game folders (Games, Program Files, SteamLibrary, ...). |
| Launching | Steam via `steam://rungameid` (launch options via `steam://run/<id>//<args>/`), Epic via its launcher URI, standalone exes with args and working directory, missing-exe error, last-played tracking. |
| Minecraft | Detects versions, lists Fabric game/loader versions from meta.fabricmc.net, installs Fabric by writing the official profile JSON, creates profiles (own folder, mods, RAM, Java, JVM args), runs pre-launch checks, writes the profile into the official launcher and opens it. |
| Mods | Modrinth API: featured performance mods, search, per-version compatibility, required-dependency install, SHA-1 verification, only `.jar` files from `cdn.modrinth.com`, enable/disable (`.jar.disabled`), remove, dependency + version-constraint checks read from each jar's `fabric.mod.json`. |
| Downloads | Progress, speed, cancel, retry, failure reasons. |
| Optimizer | Minecraft `options.txt` + profile RAM, Fortnite `GameUserSettings.ini` (refuses while Fortnite runs), saved launch options, Windows power plan, Game Mode. Every change is backed up; **Restore** and **Reset** undo them. If nothing changed, it says so. |
| Other | Live CPU/RAM, GPU, startup apps (read only), top processes (security/system processes protected), autostart toggle, clear cache, restore default settings. |

## Honest limitations

- **Minecraft Play opens the official launcher with the Rift profile prepared**; you press Play there. The launcher has no supported command-line "start this profile", and Rift will not handle Microsoft authentication itself. Close the launcher before pressing Play in Rift (it rewrites `launcher_profiles.json` on exit; a one-time backup `launcher_profiles.json.rift.bak` is kept).
- Fabric install writes the loader profile; the launcher downloads vanilla files/libraries on first run.
- **CurseForge is not implemented** - it needs an API key. Modrinth needs none.
- **No FPS counter**: it needs a frame-capture tool such as PresentMon. Rift shows none instead of a fake number.
- Roblox launches via the `roblox://` handler (best effort, untested). Roblox, VALORANT, GTA, Terraria get recommendations and launch options only; no file edits, because the changes are either unsupported by the publisher or risk anti-cheat flags.
- Minecraft Bedrock: launch only (it has no Fabric).
- Game "art" is generated tiles (no logos or copyrighted artwork are bundled).
- "Mod download directory" is implemented as the folder that holds profiles and their mods.

## Layout

```
src/                 React UI (pages/, components/, api.ts = typed Rust commands)
src-tauri/src/
  main.rs            app state + command registry
  store.rs           settings, games, profiles, backups (rift.json in %APPDATA%\app.rift.launcher)
  games.rs           scanners + launching
  minecraft.rs       Fabric, profiles, play checks
  mods.rs            Modrinth, downloads, local mod parsing
  optimizer.rs       system info, backups, undo
```

Data lives in `%APPDATA%\app.rift.launcher\` (`rift.json`, `backups\`, `instances\`).
