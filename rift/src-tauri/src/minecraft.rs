use crate::store::{Instance, Settings};
use crate::util::*;
use crate::Shared;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::State;

const UA: &str = "Rift/0.1.0 (local game launcher)";

pub fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap()
}

pub fn mc_dir(s: &Settings) -> PathBuf {
    if !s.minecraft_dir.trim().is_empty() {
        return PathBuf::from(s.minecraft_dir.trim());
    }
    env_path("APPDATA").unwrap_or_default().join(".minecraft")
}

pub fn instances_root(st: &crate::AppState, s: &Settings) -> PathBuf {
    if !s.instances_dir.trim().is_empty() {
        PathBuf::from(s.instances_dir.trim())
    } else {
        st.data_dir.join("instances")
    }
}

pub fn instance_dir(st: &crate::AppState, s: &Settings, inst: &Instance) -> PathBuf {
    if !inst.dir.is_empty() {
        PathBuf::from(&inst.dir)
    } else {
        instances_root(st, s).join(&inst.id)
    }
}

pub fn fabric_version_id(inst: &Instance) -> String {
    format!("fabric-loader-{}-{}", inst.loader_version, inst.mc_version)
}

#[derive(Serialize)]
pub struct McVersion {
    pub id: String,
    pub kind: String, // vanilla | fabric | other
}

#[derive(Serialize)]
pub struct McInfo {
    pub dir: String,
    pub exists: bool,
    pub versions: Vec<McVersion>,
    pub launcher: String, // "exe path" | "store" | ""
    pub java: String,
}

fn launcher_target() -> Option<String> {
    for key in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(pf) = env_path(key) {
            let p = pf.join("Minecraft Launcher").join("MinecraftLauncher.exe");
            if p.exists() {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    if let Some(la) = env_path("LOCALAPPDATA") {
        if la.join("Packages").join("Microsoft.4297127D64EC6_8wekyb3d8bbwe").exists() {
            return Some("store".into());
        }
    }
    None
}

pub fn open_launcher() -> Result<(), String> {
    match launcher_target().as_deref() {
        Some("store") => open_uri("shell:AppsFolder\\Microsoft.4297127D64EC6_8wekyb3d8bbwe!Minecraft"),
        Some(exe) => cmd(exe).spawn().map(|_| ()).map_err(e),
        None => Err("The official Minecraft Launcher was not found. Install it from minecraft.net, sign in with your Microsoft account, then try again.".into()),
    }
}

#[tauri::command]
pub fn mc_info(st: State<'_, Shared>) -> McInfo {
    let settings = st.read(|s| s.settings.clone());
    let dir = mc_dir(&settings);
    let mut versions = vec![];
    if let Ok(rd) = std::fs::read_dir(dir.join("versions")) {
        for ent in rd.flatten() {
            let id = ent.file_name().to_string_lossy().to_string();
            if !ent.path().join(format!("{}.json", id)).exists() {
                continue;
            }
            let kind = if id.starts_with("fabric-loader-") {
                "fabric"
            } else if ent.path().join(format!("{}.jar", id)).exists() {
                "vanilla"
            } else {
                "other"
            };
            versions.push(McVersion { id, kind: kind.into() });
        }
    }
    versions.sort_by(|a, b| b.id.cmp(&a.id));
    let java = if settings.java_path.trim().is_empty() {
        run_out("where", &["java"]).lines().next().unwrap_or("").trim().to_string()
    } else {
        settings.java_path.clone()
    };
    McInfo {
        exists: dir.exists(),
        dir: dir.to_string_lossy().to_string(),
        versions,
        launcher: launcher_target().unwrap_or_default(),
        java,
    }
}

#[tauri::command]
pub async fn fabric_game_versions() -> Result<Vec<String>, String> {
    let v: Value = http()
        .get("https://meta.fabricmc.net/v2/versions/game")
        .send().await.map_err(e)?
        .error_for_status().map_err(e)?
        .json().await.map_err(e)?;
    Ok(v.as_array()
        .map(|a| {
            a.iter()
                .filter(|x| x["stable"].as_bool() == Some(true))
                .filter_map(|x| x["version"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default())
}

#[tauri::command]
pub async fn fabric_loader_versions(mc_version: String) -> Result<Vec<String>, String> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{}", mc_version);
    let v: Value = http().get(url).send().await.map_err(e)?.error_for_status().map_err(e)?.json().await.map_err(e)?;
    Ok(v.as_array()
        .map(|a| {
            a.iter()
                .filter(|x| x["loader"]["stable"].as_bool() == Some(true))
                .filter_map(|x| x["loader"]["version"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default())
}

/// Installs the Fabric loader the same way the official Fabric installer does:
/// it writes the version profile JSON into <.minecraft>/versions/. The official launcher
/// downloads the vanilla parent version and libraries the first time the profile runs.
async fn install_fabric_inner(mc_dir: &Path, mc: &str, loader: &str) -> Result<String, String> {
    let id = format!("fabric-loader-{}-{}", loader, mc);
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json", mc, loader);
    let text = http().get(url).send().await.map_err(e)?.error_for_status().map_err(e)?.text().await.map_err(e)?;
    let parsed: Value = serde_json::from_str(&text).map_err(|_| "Fabric returned an invalid profile.".to_string())?;
    if parsed["id"].as_str() != Some(&id) {
        return Err("Fabric profile did not match the requested versions.".into());
    }
    let vdir = mc_dir.join("versions").join(&id);
    std::fs::create_dir_all(&vdir).map_err(e)?;
    std::fs::write(vdir.join(format!("{}.json", id)), text).map_err(e)?;
    Ok(id)
}

#[tauri::command]
pub async fn install_fabric(st: State<'_, Shared>, mc_version: String, loader_version: String) -> Result<String, String> {
    let dir = mc_dir(&st.read(|s| s.settings.clone()));
    if !dir.exists() {
        return Err(format!("Minecraft folder not found: {}\nRun the official launcher once, or set the path in Settings.", dir.display()));
    }
    install_fabric_inner(&dir, &mc_version, &loader_version).await
}

#[tauri::command]
pub async fn create_instance(
    st: State<'_, Shared>,
    name: String,
    mc_version: String,
    loader_version: String,
) -> Result<Instance, String> {
    let settings = st.read(|s| s.settings.clone());
    if name.trim().is_empty() {
        return Err("Give the profile a name.".into());
    }
    let dir = mc_dir(&settings);
    if !dir.exists() {
        return Err(format!("Minecraft folder not found: {}", dir.display()));
    }
    install_fabric_inner(&dir, &mc_version, &loader_version).await?;
    let id = format!("{}-{}", slug(&name), now() % 100000);
    let inst = Instance {
        id: id.clone(),
        name: name.trim().into(),
        mc_version,
        loader_version,
        ram_mb: settings.ram_mb,
        jvm_args: String::new(),
        java_path: String::new(),
        dir: String::new(),
    };
    let idir = instance_dir(&st, &settings, &inst);
    std::fs::create_dir_all(idir.join("mods")).map_err(e)?;
    let mut inst = inst;
    inst.dir = idir.to_string_lossy().to_string();
    st.mutate(|s| {
        s.instances.push(inst.clone());
        s.selected_instance = id;
    });
    Ok(inst)
}

#[tauri::command]
pub fn save_instance(st: State<'_, Shared>, inst: Instance) -> Result<(), String> {
    if inst.ram_mb < 1024 {
        return Err("Allocate at least 1024 MB.".into());
    }
    st.mutate(|s| {
        if let Some(x) = s.instances.iter_mut().find(|x| x.id == inst.id) {
            x.name = inst.name;
            x.ram_mb = inst.ram_mb;
            x.jvm_args = inst.jvm_args;
            x.java_path = inst.java_path;
        }
    });
    Ok(())
}

#[tauri::command]
pub fn select_instance(st: State<'_, Shared>, id: String) {
    st.mutate(|s| s.selected_instance = id);
}

#[tauri::command]
pub fn delete_instance(st: State<'_, Shared>, id: String, delete_files: bool) -> Result<(), String> {
    let settings = st.read(|s| s.settings.clone());
    let inst = st.read(|s| s.instances.iter().find(|i| i.id == id).cloned()).ok_or("Unknown profile.")?;
    if delete_files {
        let d = instance_dir(&st, &settings, &inst);
        // Only ever delete inside Rift's own instances folder.
        if d.starts_with(instances_root(&st, &settings)) {
            let _ = std::fs::remove_dir_all(d);
        }
    }
    // Remove the profile Rift added to the official launcher, if present.
    let _ = edit_profiles(&mc_dir(&settings), |p| {
        p.remove(&format!("rift-{}", id));
    });
    st.mutate(|s| {
        s.instances.retain(|i| i.id != id);
        s.mod_records.retain(|m| m.instance_id != id);
        if s.selected_instance == id {
            s.selected_instance = s.instances.first().map(|i| i.id.clone()).unwrap_or_default();
        }
    });
    Ok(())
}

fn edit_profiles(mc: &Path, f: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Result<(), String> {
    let path = mc.join("launcher_profiles.json");
    let mut root: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({"profiles": {}, "version": 3}));
    if !root["profiles"].is_object() {
        root["profiles"] = json!({});
    }
    // One-time backup of the user's original launcher profiles.
    let bak = mc.join("launcher_profiles.json.rift.bak");
    if path.exists() && !bak.exists() {
        let _ = std::fs::copy(&path, &bak);
    }
    f(root["profiles"].as_object_mut().unwrap());
    std::fs::write(&path, serde_json::to_string_pretty(&root).map_err(e)?).map_err(e)
}

#[derive(Serialize)]
pub struct Check {
    pub label: String,
    pub ok: bool,
    pub detail: String,
    pub blocking: bool,
}

#[derive(Serialize)]
pub struct PlayResult {
    pub started: bool,
    pub checks: Vec<Check>,
    pub message: String,
}

#[tauri::command]
pub fn mc_play(st: State<'_, Shared>, instance_id: String, force: bool) -> PlayResult {
    let (settings, inst) = st.read(|s| (s.settings.clone(), s.instances.iter().find(|i| i.id == instance_id).cloned()));
    let mut checks: Vec<Check> = vec![];
    let fail = |checks: Vec<Check>, msg: &str| PlayResult { started: false, checks, message: msg.into() };
    let Some(inst) = inst else {
        return fail(checks, "Select or create a profile first.");
    };
    let dir = mc_dir(&settings);

    // 1. Minecraft installation
    let mc_ok = dir.exists();
    checks.push(Check { label: "Minecraft installation".into(), ok: mc_ok, detail: dir.to_string_lossy().to_string(), blocking: true });

    // 2. Fabric installation
    let fid = fabric_version_id(&inst);
    let fab_ok = dir.join("versions").join(&fid).join(format!("{}.json", fid)).exists();
    checks.push(Check { label: "Fabric loader".into(), ok: fab_ok, detail: fid.clone(), blocking: true });

    // Vanilla parent (informational: the official launcher downloads it if missing)
    let van_ok = dir.join("versions").join(&inst.mc_version).join(format!("{}.jar", inst.mc_version)).exists();
    checks.push(Check {
        label: format!("Minecraft {} files", inst.mc_version),
        ok: van_ok,
        detail: if van_ok { "Installed".into() } else { "Not downloaded yet - the official launcher will download them on first run.".into() },
        blocking: false,
    });

    // 3. Mods and dependencies
    let mods_dir = instance_dir(&st, &settings, &inst).join("mods");
    let mods = crate::mods::read_local_mods(&mods_dir, &inst.mc_version);
    let missing = crate::mods::missing_deps(&mods);
    checks.push(Check {
        label: "Mod dependencies".into(),
        ok: missing.is_empty(),
        detail: if missing.is_empty() {
            format!("{} mod(s) checked, nothing missing", mods.iter().filter(|m| m.enabled).count())
        } else {
            missing.iter().map(|m| format!("{} needs {}", m.mod_name, m.needs)).collect::<Vec<_>>().join("; ")
        },
        blocking: true,
    });
    let incompatible: Vec<_> = mods.iter().filter(|m| m.enabled && m.compatible == Some(false)).collect();
    checks.push(Check {
        label: "Mod / game version match".into(),
        ok: incompatible.is_empty(),
        detail: if incompatible.is_empty() {
            format!("All enabled mods declare support for {}", inst.mc_version)
        } else {
            incompatible.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(", ") + " may not support this Minecraft version"
        },
        blocking: true,
    });

    let blocked = checks.iter().any(|c| c.blocking && !c.ok);
    if blocked && !(force && mc_ok && fab_ok) {
        return fail(checks, "Fix the failed checks above before launching (or launch anyway if only mod checks failed).");
    }

    // 4. Write the profile into the official launcher and open it.
    let java_args = format!(
        "-Xmx{}M -Xms{}M {}",
        inst.ram_mb,
        (inst.ram_mb / 4).max(512),
        inst.jvm_args.trim()
    );
    let java = if !inst.java_path.trim().is_empty() { inst.java_path.clone() } else { settings.java_path.clone() };
    let gamedir = instance_dir(&st, &settings, &inst);
    let pid = format!("rift-{}", inst.id);
    let name = format!("Rift - {}", inst.name);
    let res = edit_profiles(&dir, |p| {
        let mut prof = json!({
            "name": name,
            "type": "custom",
            "created": chrono::Utc::now().to_rfc3339(),
            "lastUsed": chrono::Utc::now().to_rfc3339(),
            "icon": "Furnace",
            "lastVersionId": fid,
            "gameDir": gamedir.to_string_lossy(),
            "javaArgs": java_args.trim(),
        });
        if !java.trim().is_empty() {
            prof["javaDir"] = json!(java);
        }
        p.insert(pid, prof);
    });
    if let Err(err) = res {
        return fail(checks, &format!("Could not write the launcher profile: {}", err));
    }
    match open_launcher() {
        Ok(()) => {
            st.mutate(|s| s.touch_played("minecraft-java"));
            PlayResult {
                started: true,
                checks,
                message: format!("Opened the official Minecraft Launcher with the profile \"{}\" prepared. Press Play there - it uses your signed-in Microsoft account.", name),
            }
        }
        Err(err) => fail(checks, &err),
    }
}
