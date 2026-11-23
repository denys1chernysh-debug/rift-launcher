use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub accent: String,
    pub theme: String, // "dark" | "black"
    pub scan_folders: Vec<String>,
    pub minecraft_dir: String,
    pub java_path: String,
    pub ram_mb: u32,
    pub instances_dir: String,
    pub launch_behavior: String, // "keep" | "minimize" | "close"
    pub notifications: bool,
    pub start_with_windows: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            accent: "#873BFF".into(),
            theme: "dark".into(),
            scan_folders: vec![],
            minecraft_dir: String::new(),
            java_path: String::new(),
            ram_mb: 4096,
            instances_dir: String::new(),
            launch_behavior: "keep".into(),
            notifications: true,
            start_with_windows: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Game {
    pub id: String,
    pub name: String,
    /// steam | epic | exe | uri | uwp | minecraft-java
    pub kind: String,
    pub path: String,
    pub exe: String,
    pub uri: String,
    pub args: String,
    pub catalog_key: String,
    pub last_played: i64,
    pub added: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader_version: String,
    pub ram_mb: u32,
    pub jvm_args: String,
    pub java_path: String,
    pub dir: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ModRecord {
    pub instance_id: String,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub file: String,
    pub version: String,
    pub ts: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Undo {
    RestoreFile { path: String, backup: Option<String> },
    PowerScheme { guid: String },
    Reg { key: String, name: String, prev: Option<String> },
    GameArgs { game_id: String, prev: String },
    InstanceRam { id: String, prev: u32 },
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Backup {
    pub id: String,
    pub ts: i64,
    pub label: String,
    pub game_id: String,
    pub undo: Vec<Undo>,
    pub restored: bool,
}

impl Default for Undo {
    fn default() -> Self {
        Undo::PowerScheme { guid: String::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Store {
    pub settings: Settings,
    pub games: Vec<Game>,
    pub ignored: Vec<String>,
    pub instances: Vec<Instance>,
    pub selected_instance: String,
    pub mod_records: Vec<ModRecord>,
    pub backups: Vec<Backup>,
}

impl Store {
    fn file(dir: &Path) -> PathBuf {
        dir.join("rift.json")
    }

    pub fn load(dir: &Path) -> Store {
        std::fs::read_to_string(Self::file(dir))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) {
        let tmp = dir.join("rift.json.tmp");
        if let Ok(text) = serde_json::to_string_pretty(self) {
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, Self::file(dir));
            }
        }
    }
}
