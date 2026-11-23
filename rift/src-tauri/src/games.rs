use crate::store::{Game, Store};
use crate::util::*;
use crate::Shared;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

// ---------- helpers ----------

fn catalog_key(name: &str, appid: &str) -> String {
    match appid {
        "730" => return "cs2".into(),
        "271590" => return "gta5".into(),
        "12210" | "12220" => return "gta4".into(),
        "105600" => return "terraria".into(),
        _ => {}
    }
    let n = name.to_lowercase();
    let key: &str = if n.contains("fortnite") {
        "fortnite"
    } else if n.contains("roblox") {
        "roblox"
    } else if n.contains("valorant") {
        "valorant"
    } else if n.contains("counter-strike 2") {
        "cs2"
    } else if n.contains("grand theft auto v") && !n.contains("iv") {
        "gta5"
    } else if n.contains("grand theft auto iv") {
        "gta4"
    } else if n.contains("terraria") {
        "terraria"
    } else {
        ""
    };
    key.into()
}

fn mk(kind: &str, name: &str, path: &str, exe: &str, uri: &str, args: &str, appid: &str) -> Game {
    let key = catalog_key(name, appid);
    // Games that map to a catalog entry get a stable id so the catalog page finds them.
    let id = if key.is_empty() { format!("{}:{}", kind, slug(name)) } else { key.clone() };
    Game {
        id,
        name: name.into(),
        kind: kind.into(),
        path: path.into(),
        exe: exe.into(),
        uri: uri.into(),
        args: args.into(),
        catalog_key: key,
        last_played: 0,
        added: now(),
    }
}

fn vdf_pairs(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.trim().split('"').collect();
            if parts.len() >= 4 && parts[0].is_empty() {
                Some((parts[1].to_string(), parts[3].replace("\\\\", "\\")))
            } else {
                None
            }
        })
        .collect()
}

// ---------- detectors ----------

fn steam_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(p) = k.get_value::<String, _>("SteamPath") {
                let p = PathBuf::from(p);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    for key in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(pf) = env_path(key) {
            let p = pf.join("Steam");
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn detect_steam() -> Vec<Game> {
    let mut out = vec![];
    let Some(root) = steam_root() else { return out };
    let mut libs = vec![root.clone()];
    if let Ok(t) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for (k, v) in vdf_pairs(&t) {
            if k == "path" {
                let p = PathBuf::from(v);
                if p.exists() && !libs.contains(&p) {
                    libs.push(p);
                }
            }
        }
    }
    for lib in libs {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else { continue };
        for ent in rd.flatten() {
            let fname = ent.file_name().to_string_lossy().to_string();
            if !(fname.starts_with("appmanifest_") && fname.ends_with(".acf")) {
                continue;
            }
            let Ok(t) = std::fs::read_to_string(ent.path()) else { continue };
            let pairs = vdf_pairs(&t);
            let get = |k: &str| pairs.iter().find(|(a, _)| a == k).map(|(_, b)| b.clone()).unwrap_or_default();
            let (appid, name, dir) = (get("appid"), get("name"), get("installdir"));
            if appid.is_empty() || name.is_empty() {
                continue;
            }
            if name.contains("Steamworks") || name.contains("Proton") || name.contains("Steam Linux Runtime") {
                continue;
            }
            let path = apps.join("common").join(dir);
            if !path.exists() {
                continue;
            }
            out.push(mk(
                "steam",
                &name,
                &path.to_string_lossy(),
                "",
                &format!("steam://rungameid/{}", appid),
                "",
                &appid,
            ));
        }
    }
    out
}

fn detect_epic() -> Vec<Game> {
    let mut out = vec![];
    let Some(pd) = env_path("ProgramData") else { return out };
    let dir = pd.join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests");
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for ent in rd.flatten() {
        if ent.path().extension().and_then(|x| x.to_str()) != Some("item") {
            continue;
        }
        let Ok(t) = std::fs::read_to_string(ent.path()) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) else { continue };
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        let (name, loc, exe) = (s("DisplayName"), s("InstallLocation"), s("LaunchExecutable"));
        if name.is_empty() || exe.is_empty() || !Path::new(&loc).exists() {
            continue;
        }
        let uri = format!(
            "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch&silent=true",
            s("CatalogNamespace"),
            s("CatalogItemId"),
            s("AppName")
        );
        let full_exe = Path::new(&loc).join(&exe).to_string_lossy().to_string();
        out.push(mk("epic", &name, &loc, &full_exe, &uri, "", ""));
    }
    out
}

fn detect_minecraft(settings: &crate::store::Settings) -> Vec<Game> {
    let mut out = vec![];
    let mc = crate::minecraft::mc_dir(settings);
    if mc.join("versions").exists() || mc.join("launcher_profiles.json").exists() {
        let mut g = mk("minecraft-java", "Minecraft Java Edition", &mc.to_string_lossy(), "", "", "", "");
        g.id = "minecraft-java".into();
        g.catalog_key = "minecraft-java".into();
        out.push(g);
    }
    if let Some(la) = env_path("LOCALAPPDATA") {
        if la.join("Packages").join("Microsoft.MinecraftUWP_8wekyb3d8bbwe").exists() {
            let mut g = mk(
                "uwp",
                "Minecraft Bedrock Edition",
                &la.join("Packages").join("Microsoft.MinecraftUWP_8wekyb3d8bbwe").to_string_lossy(),
                "",
                "shell:AppsFolder\\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App",
                "",
                "",
            );
            g.id = "minecraft-bedrock".into();
            g.catalog_key = "minecraft-bedrock".into();
            out.push(g);
        }
    }
    out
}

fn detect_roblox() -> Vec<Game> {
    let Some(la) = env_path("LOCALAPPDATA") else { return vec![] };
    let versions = la.join("Roblox").join("Versions");
    let Ok(rd) = std::fs::read_dir(&versions) else { return vec![] };
    for ent in rd.flatten() {
        if ent.path().join("RobloxPlayerBeta.exe").exists() {
            // Roblox is launched through its registered roblox:// protocol handler.
            return vec![mk("uri", "Roblox", &ent.path().to_string_lossy(), "", "roblox://", "", "")];
        }
    }
    vec![]
}

fn detect_valorant() -> Vec<Game> {
    let Some(pd) = env_path("ProgramData") else { return vec![] };
    let riot = pd.join("Riot Games");
    if !riot.join("Metadata").join("valorant.live").exists() {
        return vec![];
    }
    let Ok(t) = std::fs::read_to_string(riot.join("RiotClientInstalls.json")) else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) else { return vec![] };
    let rc = v["rc_default"].as_str().or(v["rc_live"].as_str()).unwrap_or("");
    if rc.is_empty() || !Path::new(rc).exists() {
        return vec![];
    }
    let dir = Path::new(rc).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    vec![mk("exe", "VALORANT", &dir, rc, "", "--launch-product=valorant --launch-patchline=live", "")]
}

const SKIP_DIRS: &[&str] = &[
    "windows", "$recycle.bin", "system volume information", "programdata", "appdata", "recovery",
    "windowsapps", "common files", "windows defender", "windows nt", "microsoft", "intel", "nvidia",
    "nvidia corporation", "amd", "dotnet", "msbuild", "reference assemblies", "internet explorer",
    "windowspowershell", "steam", "epic games launcher", "microsoft shared",
];
const SKIP_EXE: &[&str] = &[
    "unins", "crash", "setup", "redist", "vc_", "dxsetup", "installer", "updater", "helper", "report",
    "unitycrashhandler", "dotnet", "vcredist", "cleanup", "uninstall",
];

fn best_exe(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(u64, PathBuf)> = None;
    for ent in walkdir::WalkDir::new(dir).max_depth(2).into_iter().filter_map(|x| x.ok()) {
        let p = ent.path();
        if p.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("exe")) != Some(true) {
            continue;
        }
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if SKIP_EXE.iter().any(|s| name.contains(s)) {
            continue;
        }
        let size = ent.metadata().map(|m| m.len()).unwrap_or(0);
        if size < 1_000_000 {
            continue;
        }
        if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
            best = Some((size, p.to_path_buf()));
        }
    }
    best.map(|(_, p)| p)
}

/// Scan one root: every immediate sub-folder is treated as a candidate game folder.
fn scan_root(root: &Path, is_drive_root: bool) -> Vec<Game> {
    let mut out = vec![];
    // On a drive root only look inside well-known game folders - never the whole disk.
    let candidates: Vec<PathBuf> = if is_drive_root {
        let names = ["Games", "Program Files", "Program Files (x86)", "SteamLibrary", "Epic Games", "XboxGames", "GOG Games", "Riot Games"];
        names.iter().map(|n| root.join(n)).filter(|p| p.exists()).collect()
    } else {
        vec![root.to_path_buf()]
    };
    for base in candidates {
        let Ok(rd) = std::fs::read_dir(&base) else { continue };
        for ent in rd.flatten() {
            let Ok(ft) = ent.file_type() else { continue };
            if !ft.is_dir() {
                continue;
            }
            let name = ent.file_name().to_string_lossy().to_string();
            if SKIP_DIRS.contains(&name.to_lowercase().as_str()) || name.starts_with('.') {
                continue;
            }
            if let Some(exe) = best_exe(&ent.path()) {
                out.push(mk("exe", &name, &ent.path().to_string_lossy(), &exe.to_string_lossy(), "", "", ""));
            }
        }
    }
    out
}

// ---------- commands ----------

#[derive(Serialize)]
pub struct ScanResult {
    pub found: usize,
    pub added: Vec<String>,
}

#[tauri::command]
pub fn list_drives() -> Vec<String> {
    (b'A'..=b'Z')
        .map(|c| format!("{}:\\", c as char))
        .filter(|d| Path::new(d).exists())
        .collect()
}

#[tauri::command]
pub async fn scan_games(st: State<'_, Shared>, folders: Vec<String>, drives: Vec<String>) -> Result<ScanResult, String> {
    let settings = st.read(|s| s.settings.clone());
    let (folders2, drives2) = (folders.clone(), drives.clone());
    let found: Vec<Game> = tauri::async_runtime::spawn_blocking(move || {
        let mut all = vec![];
        all.extend(detect_steam());
        all.extend(detect_epic());
        all.extend(detect_minecraft(&settings));
        all.extend(detect_roblox());
        all.extend(detect_valorant());
        for d in drives2 {
            all.extend(scan_root(Path::new(&d), true));
        }
        for f in folders2 {
            all.extend(scan_root(Path::new(&f), false));
        }
        all
    })
    .await
    .map_err(e)?;

    let total = found.len();
    let added = st.mutate(|s| {
        let mut added = vec![];
        for g in found {
            if s.ignored.contains(&g.id) || s.games.iter().any(|x| x.id == g.id) {
                continue;
            }
            // A folder-scanned exe that duplicates a Steam/Epic game by path is not added twice.
            if s.games.iter().any(|x| !x.path.is_empty() && x.path.eq_ignore_ascii_case(&g.path)) {
                continue;
            }
            added.push(g.name.clone());
            s.games.push(g);
        }
        added
    });
    Ok(ScanResult { found: total, added })
}

#[tauri::command]
pub fn add_game_path(st: State<'_, Shared>, path: String) -> Result<Game, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("That path does not exist.".into());
    }
    let g = if p.is_dir() {
        let exe = best_exe(&p).ok_or("No game executable found in that folder.")?;
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        mk("exe", &name, &p.to_string_lossy(), &exe.to_string_lossy(), "", "", "")
    } else {
        let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let dir = p.parent().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        mk("exe", &name, &dir, &p.to_string_lossy(), "", "", "")
    };
    st.mutate(|s| {
        s.ignored.retain(|i| i != &g.id);
        if let Some(old) = s.games.iter_mut().find(|x| x.id == g.id) {
            old.exe = g.exe.clone();
            old.path = g.path.clone();
        } else {
            s.games.push(g.clone());
        }
    });
    Ok(g)
}

#[tauri::command]
pub fn remove_game(st: State<'_, Shared>, id: String) {
    st.mutate(|s| {
        s.games.retain(|g| g.id != id);
        if !s.ignored.contains(&id) {
            s.ignored.push(id);
        }
    });
}

#[tauri::command]
pub fn set_game_args(st: State<'_, Shared>, id: String, args: String) {
    st.mutate(|s| {
        if let Some(g) = s.games.iter_mut().find(|g| g.id == id) {
            g.args = args;
        }
    });
}

fn steam_uri_with_args(uri: &str, args: &str) -> String {
    if args.trim().is_empty() {
        return uri.to_string();
    }
    let appid = uri.rsplit('/').next().unwrap_or("");
    let enc = args.trim().replace(' ', "%20").replace('"', "%22");
    format!("steam://run/{}//{}/", appid, enc)
}

pub fn launch(g: &Game) -> Result<(), String> {
    match g.kind.as_str() {
        "steam" => open_uri(&steam_uri_with_args(&g.uri, &g.args)),
        "epic" | "uri" | "uwp" => open_uri(&g.uri),
        "exe" => {
            let exe = Path::new(&g.exe);
            if !exe.exists() {
                return Err(format!("The executable no longer exists:\n{}", g.exe));
            }
            let mut c = cmd(&g.exe);
            c.args(split_args(&g.args));
            if let Some(dir) = exe.parent() {
                c.current_dir(dir);
            }
            c.spawn().map_err(|x| format!("Could not start the game: {}", x))?;
            Ok(())
        }
        "minecraft-java" => crate::minecraft::open_launcher(),
        other => Err(format!("Unsupported game type: {}", other)),
    }
}

#[tauri::command]
pub fn launch_game(st: State<'_, Shared>, id: String) -> Result<(), String> {
    let g = st
        .read(|s| s.games.iter().find(|g| g.id == id).cloned())
        .ok_or("This game is not in your library. Scan for it or add it manually.")?;
    launch(&g)?;
    st.mutate(|s| {
        if let Some(x) = s.games.iter_mut().find(|x| x.id == id) {
            x.last_played = now();
        }
    });
    Ok(())
}

impl Store {
    pub fn touch_played(&mut self, id: &str) {
        if let Some(x) = self.games.iter_mut().find(|x| x.id == id) {
            x.last_played = now();
        }
    }
}
