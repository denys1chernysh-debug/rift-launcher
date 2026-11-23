import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  accent: string;
  theme: "dark" | "black";
  scan_folders: string[];
  minecraft_dir: string;
  java_path: string;
  ram_mb: number;
  instances_dir: string;
  launch_behavior: "keep" | "minimize" | "close";
  notifications: boolean;
  start_with_windows: boolean;
}
export interface Game {
  id: string; name: string; kind: string; path: string; exe: string; uri: string;
  args: string; catalog_key: string; last_played: number; added: number;
}
export interface Instance {
  id: string; name: string; mc_version: string; loader_version: string;
  ram_mb: number; jvm_args: string; java_path: string; dir: string;
}
export interface ModRecord {
  instance_id: string; project_id: string; slug: string; title: string; file: string; version: string; ts: number;
}
export type Undo = { type: string; [k: string]: unknown };
export interface Backup { id: string; ts: number; label: string; game_id: string; undo: Undo[]; restored: boolean }
export interface Store {
  settings: Settings; games: Game[]; ignored: string[]; instances: Instance[];
  selected_instance: string; mod_records: ModRecord[]; backups: Backup[];
}
export interface McInfo {
  dir: string; exists: boolean; versions: { id: string; kind: string }[]; launcher: string; java: string;
}
export interface LocalMod {
  file: string; enabled: boolean; id: string; name: string; version: string; description: string;
  depends: string[]; mc_constraint: string; compatible: boolean | null;
}
export interface MissingDep { mod_name: string; needs: string }
export interface Check { label: string; ok: boolean; detail: string; blocking: boolean }
export interface PlayResult { started: boolean; checks: Check[]; message: string }
export interface DownloadItem {
  id: string; name: string; slug: string; instance_id: string;
  status: "queued" | "downloading" | "done" | "failed" | "cancelled";
  received: number; total: number; speed: number; error: string; ts: number;
}
export interface SysLive { cpu_name: string; cores: number; cpu_usage: number; ram_total_mb: number; ram_used_mb: number }
export interface SysStatic {
  gpus: string[]; power_plan: string; power_guid: string; game_mode: boolean | null;
  startup: { name: string; command: string; location: string }[];
}
export interface Proc { pid: number; name: string; mem_mb: number; protected: boolean }
export interface Opts {
  fps?: number; vsync?: boolean; fullscreen_mode?: number; particles?: boolean; graphics?: boolean;
  ram?: boolean; power_plan?: boolean; game_mode?: boolean; launch_args?: string;
}
export interface Report { backup_id: string; changes: string[]; skipped: string[] }
export interface ModrinthProject {
  slug: string; project_id?: string; id?: string; title: string; description: string;
  icon_url?: string | null; downloads: number; versions?: string[];
}
export interface Compat { slug: string; compatible: boolean; version: string; requires: string[] }

export const api = {
  state: () => invoke<Store>("get_state"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  drives: () => invoke<string[]>("list_drives"),
  scan: (folders: string[], drives: string[]) => invoke<{ found: number; added: string[] }>("scan_games", { folders, drives }),
  addGame: (path: string) => invoke<Game>("add_game_path", { path }),
  removeGame: (id: string) => invoke<void>("remove_game", { id }),
  setArgs: (id: string, args: string) => invoke<void>("set_game_args", { id, args }),
  launch: (id: string) => invoke<void>("launch_game", { id }),
  mcInfo: () => invoke<McInfo>("mc_info"),
  fabricGames: () => invoke<string[]>("fabric_game_versions"),
  fabricLoaders: (mcVersion: string) => invoke<string[]>("fabric_loader_versions", { mcVersion }),
  installFabric: (mcVersion: string, loaderVersion: string) => invoke<string>("install_fabric", { mcVersion, loaderVersion }),
  createInstance: (name: string, mcVersion: string, loaderVersion: string) =>
    invoke<Instance>("create_instance", { name, mcVersion, loaderVersion }),
  saveInstance: (inst: Instance) => invoke<void>("save_instance", { inst }),
  selectInstance: (id: string) => invoke<void>("select_instance", { id }),
  deleteInstance: (id: string, deleteFiles: boolean) => invoke<void>("delete_instance", { id, deleteFiles }),
  mcPlay: (instanceId: string, force: boolean) => invoke<PlayResult>("mc_play", { instanceId, force }),
  listMods: (instanceId: string) => invoke<{ mods: LocalMod[]; missing: MissingDep[] }>("list_mods", { instanceId }),
  toggleMod: (instanceId: string, file: string) => invoke<void>("toggle_mod", { instanceId, file }),
  removeMod: (instanceId: string, file: string) => invoke<void>("remove_mod", { instanceId, file }),
  openModsFolder: (instanceId: string) => invoke<void>("open_mods_folder", { instanceId }),
  modSearch: (query: string, mcVersion: string) => invoke<ModrinthProject[]>("mod_search", { query, mcVersion }),
  modFeatured: () => invoke<ModrinthProject[]>("mod_featured"),
  modCompat: (slugs: string[], mcVersion: string) => invoke<Compat[]>("mod_compat", { slugs, mcVersion }),
  installMod: (instanceId: string, slug: string, title: string) => invoke<string>("install_mod", { instanceId, slug, title }),
  retry: (id: string) => invoke<void>("retry_download", { id }),
  cancel: (id: string) => invoke<void>("cancel_download", { id }),
  downloads: () => invoke<DownloadItem[]>("list_downloads"),
  clearDownloads: () => invoke<void>("clear_finished_downloads"),
  sysLive: () => invoke<SysLive>("system_live"),
  sysStatic: () => invoke<SysStatic>("system_static"),
  processes: () => invoke<Proc[]>("top_processes"),
  endProcess: (pid: number) => invoke<string>("end_process", { pid }),
  optimize: (gameId: string, opts: Opts) => invoke<Report>("optimize_game", { gameId, opts }),
  restore: (id: string) => invoke<string[]>("restore_backup", { id }),
  resetOpt: () => invoke<string[]>("reset_optimization"),
  autostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  clearCache: () => invoke<string>("clear_cache"),
  resetSettings: () => invoke<void>("reset_settings"),
};

export const errText = (x: unknown) => (typeof x === "string" ? x : x instanceof Error ? x.message : JSON.stringify(x));
