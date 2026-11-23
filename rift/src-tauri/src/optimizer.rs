use crate::store::{Backup, Undo};
use crate::util::*;
use crate::Shared;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use sysinfo::ProcessesToUpdate;
use tauri::State;

// ---------------- system information ----------------

#[derive(Serialize)]
pub struct SysLive {
    pub cpu_name: String,
    pub cores: usize,
    pub cpu_usage: f32,
    pub ram_total_mb: u64,
    pub ram_used_mb: u64,
}

#[tauri::command]
pub fn system_live(st: State<'_, Shared>) -> SysLive {
    let mut s = st.sys.lock().unwrap();
    s.refresh_cpu_usage();
    s.refresh_memory();
    SysLive {
        cpu_name: s.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
        cores: s.cpus().len(),
        cpu_usage: s.global_cpu_usage(),
        ram_total_mb: s.total_memory() / 1_048_576,
        ram_used_mb: s.used_memory() / 1_048_576,
    }
}

fn ps_json(script: &str) -> Vec<Value> {
    let out = run_out("powershell.exe", &["-NoProfile", "-NonInteractive", "-Command", script]);
    match serde_json::from_str::<Value>(out.trim()) {
        Ok(Value::Array(a)) => a,
        Ok(v @ Value::Object(_)) => vec![v],
        _ => vec![],
    }
}

#[derive(Serialize)]
pub struct StartupApp {
    pub name: String,
    pub command: String,
    pub location: String,
}

#[derive(Serialize)]
pub struct SysStatic {
    pub gpus: Vec<String>,
    pub power_plan: String,
    pub power_guid: String,
    pub game_mode: Option<bool>,
    pub startup: Vec<StartupApp>,
}

pub fn active_power_scheme() -> (String, String) {
    let out = run_out("powercfg", &["/getactivescheme"]);
    let guid = out.split("GUID:").nth(1).map(|r| r.trim().split_whitespace().next().unwrap_or("").to_string()).unwrap_or_default();
    let name = out.split('(').nth(1).map(|r| r.split(')').next().unwrap_or("").to_string()).unwrap_or_default();
    (name, guid)
}

fn reg_query(key: &str, name: &str) -> Option<String> {
    let out = run_out("reg", &["query", key, "/v", name]);
    out.lines().find(|l| l.contains(name)).and_then(|l| l.split_whitespace().last().map(String::from))
}

#[tauri::command]
pub async fn system_static() -> Result<SysStatic, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let gpus = ps_json("Get-CimInstance Win32_VideoController | Select-Object Name,AdapterRAM,DriverVersion | ConvertTo-Json -Compress")
            .iter()
            .filter_map(|g| {
                let n = g["Name"].as_str()?;
                let ram = g["AdapterRAM"].as_u64().filter(|r| *r > 0).map(|r| format!(" - {} MB (as reported by Windows)", r / 1_048_576)).unwrap_or_default();
                Some(format!("{}{} - driver {}", n, ram, g["DriverVersion"].as_str().unwrap_or("?")))
            })
            .collect();
        let startup = ps_json("Get-CimInstance Win32_StartupCommand | Select-Object Name,Command,Location | ConvertTo-Json -Compress")
            .iter()
            .map(|s| StartupApp {
                name: s["Name"].as_str().unwrap_or("").into(),
                command: s["Command"].as_str().unwrap_or("").into(),
                location: s["Location"].as_str().unwrap_or("").into(),
            })
            .collect();
        let (power_plan, power_guid) = active_power_scheme();
        let game_mode = reg_query("HKCU\\Software\\Microsoft\\GameBar", "AutoGameModeEnabled").map(|v| v == "0x1");
        SysStatic { gpus, power_plan, power_guid, game_mode, startup }
    })
    .await
    .map_err(e)
}

#[derive(Serialize)]
pub struct Proc {
    pub pid: u32,
    pub name: String,
    pub mem_mb: u64,
    pub protected: bool,
}

const PROTECTED: &[&str] = &[
    "system", "registry", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe",
    "svchost.exe", "dwm.exe", "explorer.exe", "msmpeng.exe", "securityhealthservice.exe", "securityhealthsystray.exe",
    "nissrv.exe", "fontdrvhost.exe", "sihost.exe", "ctfmon.exe", "taskhostw.exe", "runtimebroker.exe", "audiodg.exe",
    "rift.exe", "memory compression", "searchhost.exe", "startmenuexperiencehost.exe",
];

#[tauri::command]
pub fn top_processes(st: State<'_, Shared>) -> Vec<Proc> {
    let mut s = st.sys.lock().unwrap();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let mut v: Vec<Proc> = s
        .processes()
        .iter()
        .map(|(pid, p)| {
            let name = p.name().to_string_lossy().to_string();
            Proc { pid: pid.as_u32(), protected: PROTECTED.contains(&name.to_lowercase().as_str()), name, mem_mb: p.memory() / 1_048_576 }
        })
        .collect();
    v.sort_by(|a, b| b.mem_mb.cmp(&a.mem_mb));
    v.truncate(40);
    v
}

/// User-initiated only. Security services and core Windows processes are refused.
#[tauri::command]
pub fn end_process(st: State<'_, Shared>, pid: u32) -> Result<String, String> {
    let mut s = st.sys.lock().unwrap();
    s.refresh_processes(ProcessesToUpdate::All, true);
    let p = s.process(sysinfo::Pid::from_u32(pid)).ok_or("That process is no longer running.")?;
    let name = p.name().to_string_lossy().to_string();
    if PROTECTED.contains(&name.to_lowercase().as_str()) || name.to_lowercase().contains("defender") || name.to_lowercase().contains("vanguard") || name.to_lowercase().contains("easyanticheat") {
        return Err(format!("{} is a protected system/security process and will not be closed by Rift.", name));
    }
    if p.kill() { Ok(format!("Closed {}.", name)) } else { Err(format!("Windows refused to close {} (it may need administrator rights).", name)) }
}

// ---------------- backups & optimization ----------------

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Opts {
    pub fps: Option<u32>,
    pub vsync: Option<bool>,
    pub fullscreen_mode: Option<u32>, // Fortnite: 0 fullscreen, 1 windowed fullscreen, 2 windowed
    pub particles: bool,
    pub graphics: bool,
    pub ram: bool,
    pub power_plan: bool,
    pub game_mode: bool,
    pub launch_args: Option<String>,
}

#[derive(Serialize)]
pub struct Report {
    pub backup_id: String,
    pub changes: Vec<String>,
    pub skipped: Vec<String>,
}

fn backup_file(dir: &Path, path: &Path, n: usize) -> Result<Undo, String> {
    if path.exists() {
        std::fs::create_dir_all(dir).map_err(e)?;
        let dest = dir.join(format!("{}_{}", n, path.file_name().unwrap_or_default().to_string_lossy()));
        std::fs::copy(path, &dest).map_err(e)?;
        Ok(Undo::RestoreFile { path: path.to_string_lossy().into(), backup: Some(dest.to_string_lossy().into()) })
    } else {
        Ok(Undo::RestoreFile { path: path.to_string_lossy().into(), backup: None })
    }
}

fn kv_set(text: &str, key: &str, val: &str, sep: char) -> String {
    let mut found = false;
    let prefix = format!("{}{}", key, sep);
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| {
            if l.trim_start().starts_with(&prefix) {
                found = true;
                format!("{}{}{}", key, sep, val)
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(format!("{}{}{}", key, sep, val));
    }
    lines.join("\n") + "\n"
}

fn ini_set(text: &str, section: &str, key: &str, val: &str) -> String {
    let header = format!("[{}]", section);
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let Some(h) = lines.iter().position(|l| l.trim() == header) else {
        lines.push(header);
        lines.push(format!("{}={}", key, val));
        return lines.join("\r\n") + "\r\n";
    };
    let end = lines.iter().skip(h + 1).position(|l| l.trim_start().starts_with('[')).map(|p| p + h + 1).unwrap_or(lines.len());
    let prefix = format!("{}=", key.to_lowercase());
    if let Some(i) = (h + 1..end).find(|&i| lines[i].trim_start().to_lowercase().starts_with(&prefix)) {
        lines[i] = format!("{}={}", key, val);
    } else {
        lines.insert(h + 1, format!("{}={}", key, val));
    }
    lines.join("\r\n") + "\r\n"
}

fn process_running(exe: &str) -> bool {
    run_out("tasklist", &["/FI", &format!("IMAGENAME eq {}", exe), "/NH"]).to_lowercase().contains(&exe.to_lowercase())
}

#[tauri::command]
pub fn optimize_game(st: State<'_, Shared>, game_id: String, opts: Opts) -> Result<Report, String> {
    let (settings, game, inst) = st.read(|s| {
        (
            s.settings.clone(),
            s.games.iter().find(|g| g.id == game_id).cloned(),
            s.instances.iter().find(|i| i.id == s.selected_instance).cloned(),
        )
    });
    let bid = format!("bk-{}", now());
    let bdir = st.data_dir.join("backups").join(&bid);
    let mut undo: Vec<Undo> = vec![];
    let mut changes: Vec<String> = vec![];
    let mut skipped: Vec<String> = vec![];
    let key = game.as_ref().map(|g| g.catalog_key.clone()).unwrap_or(game_id.clone());
    let mut n = 0;

    // ---- game specific ----
    match key.as_str() {
        "minecraft-java" => {
            let target: Option<PathBuf> = match &inst {
                Some(i) => Some(crate::minecraft::instance_dir(&st, &settings, i).join("options.txt")),
                None => Some(crate::minecraft::mc_dir(&settings).join("options.txt")),
            };
            let opt = target.unwrap();
            let want_file = opts.particles || opts.graphics || opts.fps.is_some();
            if want_file {
                if !opt.exists() {
                    skipped.push(format!("options.txt not found at {} - start Minecraft once so it is created, then optimize again.", opt.display()));
                } else {
                    n += 1;
                    undo.push(backup_file(&bdir, &opt, n)?);
                    let mut text = std::fs::read_to_string(&opt).map_err(e)?;
                    if opts.particles {
                        text = kv_set(&text, "particles", "2", ':');
                        changes.push("Minecraft: particles set to Minimal".into());
                    }
                    if opts.graphics {
                        text = kv_set(&text, "graphicsMode", "0", ':');
                        text = kv_set(&text, "entityShadows", "false", ':');
                        text = kv_set(&text, "biomeBlendRadius", "1", ':');
                        text = kv_set(&text, "mipmapLevels", "2", ':');
                        changes.push("Minecraft: graphics set to Fast, entity shadows off, biome blend 1, mipmaps 2".into());
                    }
                    if let Some(f) = opts.fps {
                        text = kv_set(&text, "maxFps", &f.to_string(), ':');
                        changes.push(format!("Minecraft: max FPS set to {}", f));
                    }
                    std::fs::write(&opt, text).map_err(e)?;
                }
            }
            if opts.ram {
                match &inst {
                    Some(i) => {
                        let total = st.sys.lock().unwrap().total_memory() / 1_048_576;
                        let rec = ((total / 2) / 512 * 512).clamp(2048, 8192) as u32;
                        if rec == i.ram_mb {
                            skipped.push(format!("RAM already {} MB for this profile.", rec));
                        } else {
                            undo.push(Undo::InstanceRam { id: i.id.clone(), prev: i.ram_mb });
                            let id = i.id.clone();
                            st.mutate(|s| {
                                if let Some(x) = s.instances.iter_mut().find(|x| x.id == id) {
                                    x.ram_mb = rec;
                                }
                            });
                            changes.push(format!("Minecraft profile RAM: {} MB -> {} MB (about half of your {} MB)", i.ram_mb, rec, total));
                        }
                    }
                    None => skipped.push("RAM: create a Minecraft profile first.".into()),
                }
            }
        }
        "fortnite" => {
            let ini = env_path("LOCALAPPDATA").map(|l| l.join("FortniteGame").join("Saved").join("Config").join("WindowsClient").join("GameUserSettings.ini"));
            let wants = opts.fps.is_some() || opts.vsync.is_some() || opts.fullscreen_mode.is_some();
            if wants {
                match ini {
                    Some(p) if p.exists() => {
                        if process_running("FortniteClient-Win64-Shipping.exe") {
                            skipped.push("Fortnite is running and would overwrite the change on exit. Close Fortnite and try again.".into());
                        } else {
                            n += 1;
                            undo.push(backup_file(&bdir, &p, n)?);
                            let sec = "/Script/FortniteGame.FortGameUserSettings";
                            let mut text = std::fs::read_to_string(&p).map_err(e)?;
                            if let Some(f) = opts.fps {
                                text = ini_set(&text, sec, "FrameRateLimit", &format!("{}.000000", f));
                                changes.push(format!("Fortnite: FPS limit {}", f));
                            }
                            if let Some(v) = opts.vsync {
                                text = ini_set(&text, sec, "bUseVSync", if v { "True" } else { "False" });
                                changes.push(format!("Fortnite: VSync {}", if v { "on" } else { "off" }));
                            }
                            if let Some(m) = opts.fullscreen_mode {
                                text = ini_set(&text, sec, "FullscreenMode", &m.to_string());
                                changes.push(format!("Fortnite: window mode {}", ["Fullscreen", "Windowed Fullscreen", "Windowed"].get(m as usize).unwrap_or(&"?")));
                            }
                            std::fs::write(&p, text).map_err(e)?;
                        }
                    }
                    _ => skipped.push("Fortnite settings file not found - launch Fortnite once so it is created.".into()),
                }
            }
        }
        "cs2" | "gta5" | "gta4" | "terraria" | "roblox" | "valorant" | "minecraft-bedrock" | "" => {}
        _ => {}
    }

    // Launch arguments (Rift stores them and passes them on launch; for Steam via steam://run).
    if let Some(args) = &opts.launch_args {
        if let Some(g) = &game {
            if &g.args != args {
                undo.push(Undo::GameArgs { game_id: g.id.clone(), prev: g.args.clone() });
                let (id, a) = (g.id.clone(), args.clone());
                st.mutate(|s| {
                    if let Some(x) = s.games.iter_mut().find(|x| x.id == id) {
                        x.args = a;
                    }
                });
                changes.push(format!("Launch options saved: {}", if args.is_empty() { "(none)" } else { args }));
            } else {
                skipped.push("Launch options were already set to that value.".into());
            }
        } else {
            skipped.push("Launch options: this game is not in your library yet.".into());
        }
    }

    // ---- Windows level (opt-in, reversible) ----
    if opts.power_plan {
        let (name, guid) = active_power_scheme();
        if name.to_lowercase().contains("high performance") || name.to_lowercase().contains("ultimate") {
            skipped.push(format!("Power plan is already \"{}\".", name));
        } else if guid.is_empty() {
            skipped.push("Could not read the current power plan.".into());
        } else {
            let out = cmd("powercfg").args(["/setactive", "SCHEME_MIN"]).output().map_err(e)?;
            let (after, _) = active_power_scheme();
            if out.status.success() && after != name {
                undo.push(Undo::PowerScheme { guid });
                changes.push(format!("Power plan: {} -> {}", name, after));
            } else {
                skipped.push("Windows would not switch to the High performance plan on this PC (it may be hidden on modern-standby laptops).".into());
            }
        }
    }
    if opts.game_mode {
        let k = "HKCU\\Software\\Microsoft\\GameBar";
        let prev = reg_query(k, "AutoGameModeEnabled");
        if prev.as_deref() == Some("0x1") {
            skipped.push("Windows Game Mode is already on.".into());
        } else {
            let ok = cmd("reg").args(["add", k, "/v", "AutoGameModeEnabled", "/t", "REG_DWORD", "/d", "1", "/f"]).output().map(|o| o.status.success()).unwrap_or(false);
            if ok {
                undo.push(Undo::Reg { key: k.into(), name: "AutoGameModeEnabled".into(), prev });
                changes.push("Windows Game Mode turned on".into());
            } else {
                skipped.push("Could not change Windows Game Mode.".into());
            }
        }
    }

    if undo.is_empty() {
        let _ = std::fs::remove_dir_all(&bdir);
        return Ok(Report { backup_id: String::new(), changes, skipped });
    }
    let label = format!("{} - {} change(s)", game.map(|g| g.name).unwrap_or(game_id.clone()), changes.len());
    st.mutate(|s| s.backups.push(Backup { id: bid.clone(), ts: now(), label, game_id, undo, restored: false }));
    Ok(Report { backup_id: bid, changes, skipped })
}

fn apply_undo(st: &crate::AppState, u: &Undo) -> Result<String, String> {
    match u {
        Undo::RestoreFile { path, backup } => match backup {
            Some(b) => {
                std::fs::copy(b, path).map_err(e)?;
                Ok(format!("Restored {}", path))
            }
            None => {
                let _ = std::fs::remove_file(path);
                Ok(format!("Removed {} (it did not exist before)", path))
            }
        },
        Undo::PowerScheme { guid } => {
            cmd("powercfg").args(["/setactive", guid]).output().map_err(e)?;
            Ok("Restored previous power plan".into())
        }
        Undo::Reg { key, name, prev } => {
            match prev {
                Some(p) => {
                    let v = i64::from_str_radix(p.trim_start_matches("0x"), 16).unwrap_or(0).to_string();
                    cmd("reg").args(["add", key, "/v", name, "/t", "REG_DWORD", "/d", &v, "/f"]).output().map_err(e)?;
                }
                None => {
                    cmd("reg").args(["delete", key, "/v", name, "/f"]).output().map_err(e)?;
                }
            }
            Ok(format!("Restored registry value {}", name))
        }
        Undo::GameArgs { game_id, prev } => {
            st.mutate(|s| {
                if let Some(g) = s.games.iter_mut().find(|g| &g.id == game_id) {
                    g.args = prev.clone();
                }
            });
            Ok("Restored previous launch options".into())
        }
        Undo::InstanceRam { id, prev } => {
            st.mutate(|s| {
                if let Some(i) = s.instances.iter_mut().find(|i| &i.id == id) {
                    i.ram_mb = *prev;
                }
            });
            Ok(format!("Restored profile RAM to {} MB", prev))
        }
    }
}

fn restore_one(st: &crate::AppState, id: &str) -> Result<Vec<String>, String> {
    let b = st.read(|s| s.backups.iter().find(|b| b.id == id).cloned()).ok_or("Unknown backup.")?;
    if b.restored {
        return Err("That backup was already restored.".into());
    }
    let mut log = vec![];
    for u in b.undo.iter().rev() {
        log.push(apply_undo(st, u).unwrap_or_else(|x| format!("FAILED: {}", x)));
    }
    st.mutate(|s| {
        if let Some(x) = s.backups.iter_mut().find(|x| x.id == id) {
            x.restored = true;
        }
    });
    Ok(log)
}

#[tauri::command]
pub fn restore_backup(st: State<'_, Shared>, id: String) -> Result<Vec<String>, String> {
    restore_one(&st, &id)
}

/// Undo every change Rift has made, newest first.
#[tauri::command]
pub fn reset_optimization(st: State<'_, Shared>) -> Result<Vec<String>, String> {
    let ids: Vec<String> = st.read(|s| s.backups.iter().rev().filter(|b| !b.restored).map(|b| b.id.clone()).collect());
    if ids.is_empty() {
        return Err("Nothing to reset - Rift has not made any changes that are still applied.".into());
    }
    let mut log = vec![];
    for id in ids {
        log.extend(restore_one(&st, &id)?);
    }
    Ok(log)
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(e)?;
    let key = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    let out = if enabled {
        cmd("reg").args(["add", key, "/v", "Rift", "/t", "REG_SZ", "/d", &format!("\"{}\"", exe.display()), "/f"]).output()
    } else {
        cmd("reg").args(["delete", key, "/v", "Rift", "/f"]).output()
    }
    .map_err(e)?;
    if enabled && !out.status.success() {
        return Err("Windows would not let Rift register for startup.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn clear_cache(st: State<'_, Shared>) -> String {
    let mut freed = 0u64;
    let cache = st.data_dir.join("cache");
    freed += dir_size(&cache);
    let _ = std::fs::remove_dir_all(&cache);
    // leftover partial downloads
    let settings = st.read(|s| s.settings.clone());
    for inst in st.read(|s| s.instances.clone()) {
        let mods = crate::minecraft::instance_dir(&st, &settings, &inst).join("mods");
        if let Ok(rd) = std::fs::read_dir(mods) {
            for f in rd.flatten() {
                if f.file_name().to_string_lossy().ends_with(".part") {
                    freed += f.metadata().map(|m| m.len()).unwrap_or(0);
                    let _ = std::fs::remove_file(f.path());
                }
            }
        }
    }
    format!("Freed {:.1} MB", freed as f64 / 1_048_576.0)
}

#[tauri::command]
pub fn reset_settings(st: State<'_, Shared>) {
    st.mutate(|s| s.settings = Default::default());
}
