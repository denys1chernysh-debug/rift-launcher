use crate::minecraft::{http, instance_dir};
use crate::store::{Instance, ModRecord};
use crate::util::*;
use crate::{AppState, Shared};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering as AO};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;

const API: &str = "https://api.modrinth.com/v2";
pub const FEATURED: &[&str] = &[
    "fabric-api", "sodium", "lithium", "ferrite-core", "immediatelyfast", "entityculling", "modernfix", "iris", "moreculling",
];

// ---------------- local mod reading ----------------

#[derive(Serialize, Clone)]
pub struct LocalMod {
    pub file: String,
    pub enabled: bool,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub depends: Vec<String>,
    pub mc_constraint: String,
    pub compatible: Option<bool>,
    #[serde(skip)]
    pub provides: Vec<String>,
    #[serde(skip)]
    pub depends_map: Vec<(String, String)>,
}

#[derive(Serialize)]
pub struct MissingDep {
    pub mod_name: String,
    pub needs: String,
}

fn parse_ver(s: &str) -> Vec<i64> {
    s.split(|c: char| c == '.' || c == '-' || c == '+')
        .map(|p| p.parse::<i64>().unwrap_or(0))
        .collect()
}

fn cmp_ver(a: &[i64], b: &[i64]) -> Ordering {
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (*a.get(i).unwrap_or(&0), *b.get(i).unwrap_or(&0));
        if x != y {
            return x.cmp(&y);
        }
    }
    Ordering::Equal
}

/// Matches one Fabric version predicate (">=1.21", "~1.21", "1.21.x", "1.21.1", "*").
fn pred_match(pred: &str, mc: &str) -> bool {
    let p = pred.trim();
    if p.is_empty() || p == "*" {
        return true;
    }
    let (op, rest) = if let Some(r) = p.strip_prefix(">=") {
        (">=", r)
    } else if let Some(r) = p.strip_prefix("<=") {
        ("<=", r)
    } else if let Some(r) = p.strip_prefix('>') {
        (">", r)
    } else if let Some(r) = p.strip_prefix('<') {
        ("<", r)
    } else if let Some(r) = p.strip_prefix('~') {
        ("~", r)
    } else if let Some(r) = p.strip_prefix('^') {
        ("^", r)
    } else if let Some(r) = p.strip_prefix('=') {
        ("=", r)
    } else {
        ("=", p)
    };
    let rest = rest.trim();
    if rest.contains(|c| c == 'x' || c == 'X' || c == '*') {
        let prefix: Vec<&str> = rest.split('.').take_while(|s| !matches!(*s, "x" | "X" | "*")).collect();
        let mv: Vec<&str> = mc.split('.').collect();
        return prefix.iter().enumerate().all(|(i, s)| mv.get(i) == Some(s));
    }
    let (v, r) = (parse_ver(mc), parse_ver(rest));
    let c = cmp_ver(&v, &r);
    match op {
        ">=" => c != Ordering::Less,
        ">" => c == Ordering::Greater,
        "<=" => c != Ordering::Greater,
        "<" => c == Ordering::Less,
        "=" => c == Ordering::Equal,
        "~" | "^" => {
            let n = if op == "^" { 1 } else if r.len() > 1 { r.len() - 1 } else { 1 };
            c != Ordering::Less && (0..n).all(|i| v.get(i).unwrap_or(&0) == r.get(i).unwrap_or(&0))
        }
        _ => true,
    }
}

fn constraint_match(c: &Value, mc: &str) -> bool {
    match c {
        Value::String(s) => s.split_whitespace().all(|p| pred_match(p, mc)),
        Value::Array(a) => a.iter().any(|x| constraint_match(x, mc)),
        _ => true,
    }
}

fn constraint_text(c: &Value) -> String {
    match c {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(constraint_text).collect::<Vec<_>>().join(" or "),
        _ => String::new(),
    }
}

pub fn read_local_mods(dir: &Path, mc: &str) -> Vec<LocalMod> {
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for ent in rd.flatten() {
        let file = ent.file_name().to_string_lossy().to_string();
        let enabled = file.ends_with(".jar");
        if !(enabled || file.ends_with(".jar.disabled")) {
            continue;
        }
        let mut m = LocalMod {
            file: file.clone(),
            enabled,
            id: String::new(),
            name: file.trim_end_matches(".disabled").trim_end_matches(".jar").to_string(),
            version: String::new(),
            description: String::new(),
            depends: vec![],
            mc_constraint: String::new(),
            compatible: None,
            provides: vec![],
            depends_map: vec![],
        };
        if let Ok(f) = std::fs::File::open(ent.path()) {
            if let Ok(mut z) = zip::ZipArchive::new(f) {
                if let Ok(mut entry) = z.by_name("fabric.mod.json") {
                    let mut text = String::new();
                    if entry.read_to_string(&mut text).is_ok() {
                        // Some mods ship fabric.mod.json with a BOM or trailing commas-free JSON; strip BOM only.
                        if let Ok(v) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
                            m.id = v["id"].as_str().unwrap_or("").into();
                            if let Some(n) = v["name"].as_str() {
                                m.name = n.into();
                            }
                            m.version = v["version"].as_str().unwrap_or("").into();
                            m.description = v["description"].as_str().unwrap_or("").into();
                            if let Some(p) = v["provides"].as_array() {
                                m.provides = p.iter().filter_map(|x| x.as_str().map(String::from)).collect();
                            }
                            if let Some(d) = v["depends"].as_object() {
                                for (k, c) in d {
                                    m.depends.push(k.clone());
                                    m.depends_map.push((k.clone(), constraint_text(c)));
                                    if k == "minecraft" {
                                        m.mc_constraint = constraint_text(c);
                                        m.compatible = Some(constraint_match(c, mc));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out.push(m);
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn missing_deps(mods: &[LocalMod]) -> Vec<MissingDep> {
    let mut have: HashSet<String> = ["minecraft", "java", "fabricloader", "fabric-loader"].iter().map(|s| s.to_string()).collect();
    for m in mods.iter().filter(|m| m.enabled) {
        have.insert(m.id.clone());
        for p in &m.provides {
            have.insert(p.clone());
        }
    }
    let mut out = vec![];
    for m in mods.iter().filter(|m| m.enabled) {
        for (dep, _) in &m.depends_map {
            if !have.contains(dep) {
                out.push(MissingDep { mod_name: m.name.clone(), needs: dep.clone() });
            }
        }
    }
    out
}

fn inst_and_dir(st: &AppState, instance_id: &str) -> Result<(Instance, std::path::PathBuf), String> {
    let settings = st.read(|s| s.settings.clone());
    let inst = st
        .read(|s| s.instances.iter().find(|i| i.id == instance_id).cloned())
        .ok_or("Create a Minecraft profile first (My Games -> Minecraft Java Edition).")?;
    let dir = instance_dir(st, &settings, &inst).join("mods");
    std::fs::create_dir_all(&dir).map_err(e)?;
    Ok((inst, dir))
}

#[derive(Serialize)]
pub struct LocalModsResult {
    pub mods: Vec<LocalMod>,
    pub missing: Vec<MissingDep>,
}

#[tauri::command]
pub fn list_mods(st: State<'_, Shared>, instance_id: String) -> Result<LocalModsResult, String> {
    let (inst, dir) = inst_and_dir(&st, &instance_id)?;
    let mods = read_local_mods(&dir, &inst.mc_version);
    let missing = missing_deps(&mods);
    Ok(LocalModsResult { mods, missing })
}

fn safe_name(f: &str) -> Result<(), String> {
    if f.contains('/') || f.contains('\\') || f.contains("..") {
        return Err("Invalid file name.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_mod(st: State<'_, Shared>, instance_id: String, file: String) -> Result<(), String> {
    safe_name(&file)?;
    let (_, dir) = inst_and_dir(&st, &instance_id)?;
    let from = dir.join(&file);
    let to = if let Some(base) = file.strip_suffix(".disabled") { dir.join(base) } else { dir.join(format!("{}.disabled", file)) };
    std::fs::rename(from, to).map_err(e)
}

#[tauri::command]
pub fn remove_mod(st: State<'_, Shared>, instance_id: String, file: String) -> Result<(), String> {
    safe_name(&file)?;
    let (_, dir) = inst_and_dir(&st, &instance_id)?;
    std::fs::remove_file(dir.join(&file)).map_err(e)?;
    st.mutate(|s| s.mod_records.retain(|m| !(m.instance_id == instance_id && m.file == file)));
    Ok(())
}

#[tauri::command]
pub fn open_mods_folder(st: State<'_, Shared>, instance_id: String) -> Result<(), String> {
    let (_, dir) = inst_and_dir(&st, &instance_id)?;
    cmd("explorer.exe").arg(dir).spawn().map(|_| ()).map_err(e)
}

// ---------------- Modrinth ----------------

#[tauri::command]
pub async fn mod_search(query: String, mc_version: String) -> Result<Value, String> {
    let mut facets = vec![r#"["project_type:mod"]"#.to_string(), r#"["categories:fabric"]"#.to_string()];
    if !mc_version.is_empty() {
        facets.push(format!(r#"["versions:{}"]"#, mc_version));
    }
    let facets = format!("[{}]", facets.join(","));
    let v: Value = http()
        .get(format!("{}/search", API))
        .query(&[("query", query.as_str()), ("facets", facets.as_str()), ("limit", "20"), ("index", "relevance")])
        .send().await.map_err(e)?
        .error_for_status().map_err(e)?
        .json().await.map_err(e)?;
    Ok(v["hits"].clone())
}

#[tauri::command]
pub async fn mod_featured() -> Result<Value, String> {
    let ids = serde_json::to_string(FEATURED).map_err(e)?;
    let v: Value = http()
        .get(format!("{}/projects", API))
        .query(&[("ids", ids.as_str())])
        .send().await.map_err(e)?
        .error_for_status().map_err(e)?
        .json().await.map_err(e)?;
    // keep the curated order
    let mut arr = v.as_array().cloned().unwrap_or_default();
    arr.sort_by_key(|p| FEATURED.iter().position(|s| Some(*s) == p["slug"].as_str()).unwrap_or(99));
    Ok(Value::Array(arr))
}

#[derive(Serialize)]
pub struct Compat {
    pub slug: String,
    pub compatible: bool,
    pub version: String,
    pub requires: Vec<String>,
}

async fn best_version(project: &str, mc: &str) -> Result<Option<Value>, String> {
    let loaders = r#"["fabric"]"#;
    let gv = format!(r#"["{}"]"#, mc);
    let v: Value = http()
        .get(format!("{}/project/{}/version", API, project))
        .query(&[("loaders", loaders), ("game_versions", gv.as_str())])
        .send().await.map_err(e)?
        .error_for_status().map_err(e)?
        .json().await.map_err(e)?;
    let arr = v.as_array().cloned().unwrap_or_default();
    Ok(arr.iter().find(|x| x["version_type"] == "release").cloned().or_else(|| arr.first().cloned()))
}

/// For each slug: does a Fabric build exist for this Minecraft version, and what does it require?
#[tauri::command]
pub async fn mod_compat(slugs: Vec<String>, mc_version: String) -> Result<Vec<Compat>, String> {
    let mut out = vec![];
    for s in slugs {
        match best_version(&s, &mc_version).await {
            Ok(Some(v)) => {
                let requires = v["dependencies"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter(|d| d["dependency_type"] == "required")
                            .filter_map(|d| d["project_id"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                out.push(Compat { slug: s, compatible: true, version: v["version_number"].as_str().unwrap_or("").into(), requires });
            }
            _ => out.push(Compat { slug: s, compatible: false, version: String::new(), requires: vec![] }),
        }
    }
    Ok(out)
}

// ---------------- downloads ----------------

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DownloadItem {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub instance_id: String,
    pub status: String, // queued | downloading | done | failed | cancelled
    pub received: u64,
    pub total: u64,
    pub speed: f64,
    pub error: String,
    pub ts: i64,
}

fn push_dl(app: &AppHandle, st: &AppState, item: DownloadItem) {
    st.downloads.lock().unwrap().push(item.clone());
    let _ = app.emit("download-update", item);
}

fn upd_dl(app: &AppHandle, st: &AppState, id: &str, f: impl FnOnce(&mut DownloadItem)) {
    let mut list = st.downloads.lock().unwrap();
    if let Some(it) = list.iter_mut().find(|d| d.id == id) {
        f(it);
        let _ = app.emit("download-update", it.clone());
    }
}

#[tauri::command]
pub fn list_downloads(st: State<'_, Shared>) -> Vec<DownloadItem> {
    st.downloads.lock().unwrap().clone()
}

#[tauri::command]
pub fn clear_finished_downloads(st: State<'_, Shared>) {
    st.downloads.lock().unwrap().retain(|d| d.status == "downloading" || d.status == "queued");
}

#[tauri::command]
pub fn cancel_download(st: State<'_, Shared>, id: String) {
    if let Some(flag) = st.cancels.lock().unwrap().get(&id) {
        flag.store(true, AO::SeqCst);
    }
}

async fn fetch_file(
    app: &AppHandle,
    st: &AppState,
    id: &str,
    url: &str,
    dest: &Path,
    sha1: &str,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    // Only Modrinth's CDN is allowed, and only over https.
    let u = reqwest::Url::parse(url).map_err(e)?;
    if u.scheme() != "https" || u.host_str() != Some("cdn.modrinth.com") {
        return Err("Blocked: files may only be downloaded from cdn.modrinth.com.".into());
    }
    let resp = http()
        .get(url)
        .timeout(Duration::from_secs(600))
        .send().await.map_err(e)?
        .error_for_status().map_err(e)?;
    let total = resp.content_length().unwrap_or(0);
    let part = dest.with_extension("jar.part");
    let mut f = tokio::fs::File::create(&part).await.map_err(e)?;
    let mut stream = resp.bytes_stream();
    let mut hasher = Sha1::new();
    let (start, mut last, mut got) = (Instant::now(), Instant::now(), 0u64);
    upd_dl(app, st, id, |d| {
        d.status = "downloading".into();
        d.total = total;
    });
    while let Some(chunk) = stream.next().await {
        if cancel.load(AO::SeqCst) {
            drop(f);
            let _ = tokio::fs::remove_file(&part).await;
            return Err("cancelled".into());
        }
        let c = match chunk {
            Ok(c) => c,
            Err(x) => {
                drop(f);
                let _ = tokio::fs::remove_file(&part).await;
                return Err(e(x));
            }
        };
        hasher.update(&c);
        f.write_all(&c).await.map_err(e)?;
        got += c.len() as u64;
        if last.elapsed() > Duration::from_millis(250) {
            let speed = got as f64 / start.elapsed().as_secs_f64().max(0.001);
            upd_dl(app, st, id, |d| {
                d.received = got;
                d.speed = speed;
            });
            last = Instant::now();
        }
    }
    f.flush().await.map_err(e)?;
    drop(f);
    if !sha1.is_empty() && hex::encode(hasher.finalize()) != sha1.to_lowercase() {
        let _ = tokio::fs::remove_file(&part).await;
        return Err("Checksum mismatch - the file was discarded.".into());
    }
    tokio::fs::rename(&part, dest).await.map_err(e)?;
    upd_dl(app, st, id, |d| {
        d.received = got;
        d.total = got;
        d.status = "done".into();
    });
    Ok(())
}

fn primary_file(v: &Value) -> Option<(String, String, String)> {
    let files = v["files"].as_array()?;
    let f = files.iter().find(|f| f["primary"] == true).or_else(|| files.first())?;
    let name = f["filename"].as_str()?.to_string();
    if !name.ends_with(".jar") || name.contains('/') || name.contains('\\') || name.contains("..") {
        return None; // never write anything that is not a plain .jar
    }
    Some((name, f["url"].as_str()?.to_string(), f["hashes"]["sha1"].as_str().unwrap_or("").to_string()))
}

async fn install_one(
    app: &AppHandle,
    st: &Arc<AppState>,
    inst: &Instance,
    mods_dir: &Path,
    project: &str,
    dl_id: &str,
    visited: &mut HashSet<String>,
    depth: u8,
) -> Result<(), String> {
    let cancel = Arc::new(AtomicBool::new(false));
    st.cancels.lock().unwrap().insert(dl_id.to_string(), cancel.clone());

    let ver = best_version(project, &inst.mc_version)
        .await?
        .ok_or_else(|| format!("No Fabric build of this mod exists for Minecraft {}.", inst.mc_version))?;
    let (file, url, sha1) = primary_file(&ver).ok_or("Modrinth returned no safe .jar file for this version.")?;
    let project_id = ver["project_id"].as_str().unwrap_or(project).to_string();
    visited.insert(project_id.clone());

    // Required dependencies first.
    if depth < 3 {
        if let Some(deps) = ver["dependencies"].as_array() {
            for d in deps.iter().filter(|d| d["dependency_type"] == "required") {
                let Some(pid) = d["project_id"].as_str() else { continue };
                let already = visited.contains(pid)
                    || st.read(|s| s.mod_records.iter().any(|m| m.instance_id == inst.id && m.project_id == pid));
                if already {
                    continue;
                }
                let did = format!("dl-{}-{}", now(), pid);
                push_dl(app, st, DownloadItem {
                    id: did.clone(),
                    name: format!("Dependency {}", pid),
                    slug: pid.into(),
                    instance_id: inst.id.clone(),
                    status: "queued".into(),
                    ts: now(),
                    ..Default::default()
                });
                if let Err(err) = Box::pin(install_one(app, st, inst, mods_dir, pid, &did, visited, depth + 1)).await {
                    upd_dl(app, st, &did, |x| {
                        x.status = if err == "cancelled" { "cancelled".into() } else { "failed".into() };
                        x.error = err.clone();
                    });
                    return Err(format!("A required dependency failed to install: {}", err));
                }
            }
        }
    }

    upd_dl(app, st, dl_id, |d| d.name = file.clone());
    fetch_file(app, st, dl_id, &url, &mods_dir.join(&file), &sha1, cancel).await?;
    st.mutate(|s| {
        s.mod_records.retain(|m| !(m.instance_id == inst.id && m.project_id == project_id));
        s.mod_records.push(ModRecord {
            instance_id: inst.id.clone(),
            project_id,
            slug: project.into(),
            title: file.trim_end_matches(".jar").into(),
            file,
            version: ver["version_number"].as_str().unwrap_or("").into(),
            ts: now(),
        });
    });
    Ok(())
}

async fn run_install(app: AppHandle, st: Arc<AppState>, instance_id: String, slug: String, dl_id: String) {
    let res: Result<(), String> = async {
        let (inst, dir) = inst_and_dir(&st, &instance_id)?;
        let mut visited = HashSet::new();
        install_one(&app, &st, &inst, &dir, &slug, &dl_id, &mut visited, 0).await
    }
    .await;
    match res {
        Ok(()) => {}
        Err(err) => upd_dl(&app, &st, &dl_id, |d| {
            d.status = if err == "cancelled" { "cancelled".into() } else { "failed".into() };
            d.error = err.clone();
        }),
    }
    let _ = app.emit("mods-changed", ());
}

#[tauri::command]
pub fn install_mod(app: AppHandle, st: State<'_, Shared>, instance_id: String, slug: String, title: String) -> Result<String, String> {
    inst_and_dir(&st, &instance_id)?;
    let id = format!("dl-{}-{}", now(), slug);
    push_dl(&app, &st, DownloadItem {
        id: id.clone(),
        name: title,
        slug: slug.clone(),
        instance_id: instance_id.clone(),
        status: "queued".into(),
        ts: now(),
        ..Default::default()
    });
    let shared: Arc<AppState> = st.inner().clone();
    tauri::async_runtime::spawn(run_install(app, shared, instance_id, slug, id.clone()));
    Ok(id)
}

#[tauri::command]
pub fn retry_download(app: AppHandle, st: State<'_, Shared>, id: String) -> Result<(), String> {
    let item = st.downloads.lock().unwrap().iter().find(|d| d.id == id).cloned().ok_or("Unknown download.")?;
    upd_dl(&app, &st, &id, |d| {
        d.status = "queued".into();
        d.error.clear();
        d.received = 0;
        d.speed = 0.0;
    });
    let shared: Arc<AppState> = st.inner().clone();
    tauri::async_runtime::spawn(run_install(app, shared, item.instance_id, item.slug, id));
    Ok(())
}

pub fn new_cancel_map() -> std::sync::Mutex<HashMap<String, Arc<AtomicBool>>> {
    std::sync::Mutex::new(HashMap::new())
}
