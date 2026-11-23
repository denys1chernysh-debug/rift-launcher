import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errText, Settings as S } from "../api";
import { useApp } from "../ctx";
import { Card, Toggle } from "../components/ui";

const SWATCHES = ["#873BFF", "#3B82FF", "#14B8A6", "#F43F5E", "#F59E0B", "#22C55E"];

export default function Settings() {
  const { store, refresh, toast } = useApp();
  const s = store.settings;
  const [mc, setMc] = useState(s.minecraft_dir);
  const [java, setJava] = useState(s.java_path);
  const [inst, setInst] = useState(s.instances_dir);

  const save = async (patch: Partial<S>) => {
    try { await api.saveSettings({ ...s, ...patch }); await refresh(); } catch (e) { toast(errText(e), "err"); }
  };
  const pick = async (dir: boolean, set: (v: string) => void, key: keyof S) => {
    const f = await open({ directory: dir, multiple: false });
    if (typeof f === "string") { set(f); save({ [key]: f } as Partial<S>); }
  };

  return (
    <>
      <h1>Settings</h1>
      <Card title="Appearance">
        <div className="row"><div><b>Theme</b></div>
          <select value={s.theme} onChange={(e) => save({ theme: e.target.value as S["theme"] })}><option value="dark">Dark gray</option><option value="black">Pure black</option></select></div>
        <div className="row"><div><b>Accent color</b></div>
          <div className="inline">
            {SWATCHES.map((c) => <button key={c} className={"swatch" + (s.accent.toLowerCase() === c.toLowerCase() ? " act" : "")} style={{ background: c }} onClick={() => save({ accent: c })} aria-label={c} />)}
            <input type="color" value={s.accent} onChange={(e) => save({ accent: e.target.value })} />
          </div></div>
      </Card>

      <Card title="Launching">
        <div className="row"><div><b>When a game starts</b><p className="muted">What Rift does after pressing Play.</p></div>
          <select value={s.launch_behavior} onChange={(e) => save({ launch_behavior: e.target.value as S["launch_behavior"] })}>
            <option value="keep">Keep Rift open</option><option value="minimize">Minimize Rift</option><option value="close">Close Rift</option></select></div>
        <div className="row"><div><b>Start Rift with Windows</b><p className="muted">Adds or removes a startup entry for your user.</p></div>
          <Toggle on={s.start_with_windows} onChange={async (v) => { try { await api.autostart(v); await save({ start_with_windows: v }); } catch (e) { toast(errText(e), "err"); } }} /></div>
        <div className="row"><div><b>Notifications</b><p className="muted">Show a notification when a download finishes or fails.</p></div>
          <Toggle on={s.notifications} onChange={(v) => save({ notifications: v })} /></div>
      </Card>

      <Card title="Minecraft">
        <div className="row col"><div><b>Minecraft folder</b><p className="muted">Empty = %APPDATA%\.minecraft</p></div>
          <div className="inline wide"><input value={mc} onChange={(e) => setMc(e.target.value)} onBlur={() => save({ minecraft_dir: mc })} /><button className="btn" onClick={() => pick(true, setMc, "minecraft_dir")}>Browse</button></div></div>
        <div className="row col"><div><b>Java path (javaw.exe)</b><p className="muted">Empty = Java bundled with the launcher.</p></div>
          <div className="inline wide"><input value={java} onChange={(e) => setJava(e.target.value)} onBlur={() => save({ java_path: java })} /><button className="btn" onClick={() => pick(false, setJava, "java_path")}>Browse</button></div></div>
        <div className="row"><div><b>Default RAM for new profiles</b><p className="muted">{s.ram_mb} MB</p></div>
          <input type="range" min={1024} max={16384} step={512} value={s.ram_mb} onChange={(e) => save({ ram_mb: +e.target.value })} /></div>
        <div className="row col"><div><b>Profiles & mods folder</b><p className="muted">Where profile folders (with their mods) and downloads are stored. Empty = Rift's data folder. Applies to new profiles.</p></div>
          <div className="inline wide"><input value={inst} onChange={(e) => setInst(e.target.value)} onBlur={() => save({ instances_dir: inst })} /><button className="btn" onClick={() => pick(true, setInst, "instances_dir")}>Browse</button></div></div>
      </Card>

      <Card title="Data">
        <div className="actions">
          <button className="btn" onClick={async () => toast(await api.clearCache())}>Clear cache</button>
          <button className="btn danger" onClick={async () => { if (confirm("Restore all settings to their defaults? Your games and profiles are kept.")) { await api.resetSettings(); await refresh(); toast("Settings restored"); } }}>Restore default settings</button>
        </div>
        <p className="muted small">Everything is stored locally in Rift's data folder. There is no account and nothing is sent anywhere except mod searches/downloads to Modrinth and version lists from Fabric.</p>
      </Card>
    </>
  );
}
