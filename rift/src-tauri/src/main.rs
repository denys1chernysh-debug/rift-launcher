#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod games;
mod minecraft;
mod mods;
mod optimizer;
mod store;
mod util;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

pub struct AppState {
    pub store: Mutex<store::Store>,
    pub downloads: Mutex<Vec<mods::DownloadItem>>,
    pub cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub sys: Mutex<sysinfo::System>,
    pub data_dir: PathBuf,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub fn mutate<T>(&self, f: impl FnOnce(&mut store::Store) -> T) -> T {
        let mut s = self.store.lock().unwrap();
        let r = f(&mut s);
        s.save(&self.data_dir);
        r
    }
    pub fn read<T>(&self, f: impl FnOnce(&store::Store) -> T) -> T {
        let s = self.store.lock().unwrap();
        f(&s)
    }
}

#[tauri::command]
fn get_state(st: State<'_, Shared>) -> store::Store {
    st.read(|s| s.clone())
}

#[tauri::command]
fn save_settings(st: State<'_, Shared>, settings: store::Settings) {
    st.mutate(|s| s.settings = settings);
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&dir).ok();
            let store = store::Store::load(&dir);
            let mut sys = sysinfo::System::new();
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            app.manage::<Shared>(Arc::new(AppState {
                store: Mutex::new(store),
                downloads: Mutex::new(vec![]),
                cancels: mods::new_cancel_map(),
                sys: Mutex::new(sys),
                data_dir: dir,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_settings,
            games::list_drives,
            games::scan_games,
            games::add_game_path,
            games::remove_game,
            games::set_game_args,
            games::launch_game,
            minecraft::mc_info,
            minecraft::fabric_game_versions,
            minecraft::fabric_loader_versions,
            minecraft::install_fabric,
            minecraft::create_instance,
            minecraft::save_instance,
            minecraft::select_instance,
            minecraft::delete_instance,
            minecraft::mc_play,
            mods::list_mods,
            mods::toggle_mod,
            mods::remove_mod,
            mods::open_mods_folder,
            mods::mod_search,
            mods::mod_featured,
            mods::mod_compat,
            mods::install_mod,
            mods::retry_download,
            mods::cancel_download,
            mods::list_downloads,
            mods::clear_finished_downloads,
            optimizer::system_live,
            optimizer::system_static,
            optimizer::top_processes,
            optimizer::end_process,
            optimizer::optimize_game,
            optimizer::restore_backup,
            optimizer::reset_optimization,
            optimizer::set_autostart,
            optimizer::clear_cache,
            optimizer::reset_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rift");
}
