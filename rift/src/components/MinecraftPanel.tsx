import { useCallback, useEffect, useState } from "react";
import { api, errText, Instance, LocalMod, McInfo, MissingDep, PlayResult } from "../api";
import { useApp } from "../ctx";
import { Card, Empty, Toggle } from "./ui";

export default function MinecraftPanel() {
  const { store, refresh, toast, go } = useApp();
  const [info, setInfo] = useState<McInfo | null>(null);
  const [mods, setMods] = useState<LocalMod[]>([]);
  const [missing, setMissing] = useState<MissingDep[]>([]);
  const [games, setGames] = useState<string[]>([]);
  const [loaders, setLoaders] = useState<string[]>([]);
  const [newName, setNewName] = useState("My Fabric");
  const [newMc, setNewMc] = useState("");
  const [newLoader, setNewLoader] = useState("");
  const [busy, setBusy] = useState(false);
  const [play, setPlay] = useState<PlayResult | null>(null);
  const [draft, setDraft] = useState<Instance | null>(null);

  const inst = store.instances.find((i) => i.id === store.selected_instance) ?? store.instances[0];

  useEffect(() => { api.mcInfo().then(setInfo).catch((e) => toast(errText(e), "err")); }, [store.instances.length]);
  useEffect(() => { api.fabricGames().then((g) => { setGames(g); if (g[0]) setNewMc(g[0]); }).catch(() => toast("Could not reach meta.fabricmc.net (are you offline?)", "err")); }, []);
  useEffect(() => { if (newMc) api.fabricLoaders(newMc).then((l) => { setLoaders(l); setNewLoader(l[0] ?? ""); }).catch(() => setLoaders([])); }, [newMc]);
  useEffect(() => { setDraft(inst ? { ...inst } : null); setPlay(null); }, [inst?.id, inst?.ram_mb]);

  const loadMods = useCallback(async () => {
    if (!inst) return setMods([]);
    try {
      const r = await api.listMods(inst.id);
      setMods(r.mods); setMissing(r.missing);
    } catch (e) { toast(errText(e), "err"); }
  }, [inst?.id]);
  useEffect(() => { loadMods(); }, [loadMods]);

  const create = async () => {
    setBusy(true);
    try { await api.createInstance(newName, newMc, newLoader); await refresh(); toast("Profile created and Fabric installed"); }
    catch (e) { toast(errText(e), "err"); }
    setBusy(false);
  };

  const saveDraft = async () => {
    if (!draft) return;
    try { await api.saveInstance(draft); await refresh(); toast("Launch settings saved"); } catch (e) { toast(errText(e), "err"); }
  };

  const doPlay = async (force: boolean) => {
    if (!inst) return;
    setBusy(true);
    try {
      if (draft) await api.saveInstance(draft);
      const r = await api.mcPlay(inst.id, force);
      setPlay(r);
      if (r.started) { toast("Minecraft Launcher opened"); await refresh(); }
    } catch (e) { toast(errText(e), "err"); }
    setBusy(false);
  };

  const fabricVersions = info?.versions.filter((v) => v.kind === "fabric") ?? [];
  const onlyModChecksFailed = play && !play.started && play.checks.filter((c) => !c.ok && c.blocking).every((c) => c.label.startsWith("Mod"));

  return (
    <>
      <Card title="Installation" sub="Detected from your PC">
        {!info ? <Empty>Checking…</Empty> : (
          <div className="kv">
            <div><span>Minecraft folder</span><b>{info.dir}</b> {info.exists ? <em className="ok">found</em> : <em className="bad">not found</em>}</div>
            <div><span>Official launcher</span>{info.launcher ? <em className="ok">found</em> : <em className="bad">not found - install it from minecraft.net</em>}</div>
            <div><span>Java</span><b>{info.java || "Bundled with the launcher"}</b></div>
            <div><span>Installed versions</span><b>{info.versions.filter((v) => v.kind === "vanilla").length} vanilla</b>, <b>{fabricVersions.length} Fabric</b></div>
          </div>
        )}
        {fabricVersions.length > 0 && <p className="muted small">Fabric: {fabricVersions.map((v) => v.id).join(", ")}</p>}
      </Card>

      <Card title="Profiles" sub="Each profile is its own game folder with its own mods, Fabric version and RAM.">
        {store.instances.length === 0 && <Empty>No profiles yet. Create one below.</Empty>}
        <div className="chips">
          {store.instances.map((i) => (
            <button key={i.id} className={"chip" + (i.id === inst?.id ? " act" : "")} onClick={async () => { await api.selectInstance(i.id); refresh(); }}>
              {i.name} <small>{i.mc_version}</small>
            </button>
          ))}
        </div>
        <div className="form-row">
          <input value={newName} onChange={(e) => setNewName(e.target.value)} placeholder="Profile name" />
          <select value={newMc} onChange={(e) => setNewMc(e.target.value)}>{games.slice(0, 60).map((g) => <option key={g}>{g}</option>)}</select>
          <select value={newLoader} onChange={(e) => setNewLoader(e.target.value)}>{loaders.map((l) => <option key={l}>{l}</option>)}</select>
          <button className="btn" disabled={busy || !newMc || !newLoader} onClick={create}>Create + install Fabric</button>
        </div>
      </Card>

      {inst && draft && (
        <>
          <Card title={`Play - ${inst.name}`} sub={`Minecraft ${inst.mc_version} · Fabric loader ${inst.loader_version}`}
            right={<button className="btn primary big" disabled={busy} onClick={() => doPlay(false)}>{busy ? "Checking…" : "▶ Play"}</button>}>
            <p className="muted small">Play runs the pre-launch checks, writes this profile (RAM, Java args, game folder) into the official Minecraft Launcher, and opens it. Sign-in stays with Microsoft's launcher - Rift never sees your account.</p>
            {play && (
              <div className="report">
                {play.checks.map((c) => (
                  <div key={c.label} className={c.ok ? "ok-line" : c.blocking ? "bad-line" : "skip-line"}>
                    {c.ok ? "✔" : c.blocking ? "✖" : "•"} <b>{c.label}</b> - {c.detail}
                  </div>
                ))}
                <p className={play.started ? "ok-line" : "bad-line"}>{play.message}</p>
                {onlyModChecksFailed && <button className="btn" onClick={() => doPlay(true)}>Launch anyway</button>}
              </div>
            )}
          </Card>

          <Card title="Launch settings" right={<button className="btn" onClick={saveDraft}>Save</button>}>
            <div className="row"><div><b>RAM allocation</b><p className="muted">{draft.ram_mb} MB</p></div>
              <input type="range" min={1024} max={16384} step={512} value={draft.ram_mb} onChange={(e) => setDraft({ ...draft, ram_mb: +e.target.value })} /></div>
            <div className="row col"><div><b>Java executable</b><p className="muted">Leave empty to use the launcher's bundled Java.</p></div>
              <input value={draft.java_path} placeholder="C:\Program Files\Java\jdk-21\bin\javaw.exe" onChange={(e) => setDraft({ ...draft, java_path: e.target.value })} /></div>
            <div className="row col"><div><b>Extra JVM arguments</b></div>
              <input value={draft.jvm_args} placeholder="-XX:+UseG1GC" onChange={(e) => setDraft({ ...draft, jvm_args: e.target.value })} /></div>
            <div className="actions">
              <button className="btn danger" onClick={async () => {
                if (!confirm(`Remove profile "${inst.name}" from Rift?\nAlso delete its folder (mods and saves inside it)?`)) return;
                const del = confirm("Delete the profile folder too?\nOK = delete files, Cancel = keep the files.");
                try { await api.deleteInstance(inst.id, del); await refresh(); } catch (e) { toast(errText(e), "err"); }
              }}>Delete profile</button>
            </div>
          </Card>

          <Card title="Installed mods" sub={`${mods.length} in this profile`}
            right={<div className="inline"><button className="btn" onClick={() => api.openModsFolder(inst.id)}>Open folder</button><button className="btn primary" onClick={() => go("mods")}>Browse mods</button></div>}>
            {missing.length > 0 && (
              <div className="warn">
                <b>Missing dependencies</b>
                {missing.map((m, i) => <div key={i}>{m.mod_name} needs <code>{m.needs}</code></div>)}
                {missing.some((m) => m.needs.startsWith("fabric")) && <div>Install <b>Fabric API</b> from the Mods page.</div>}
              </div>
            )}
            {mods.length === 0 ? <Empty>No mods installed. Use Browse mods to add Sodium, Lithium and more.</Empty> : mods.map((m) => (
              <div className="row" key={m.file}>
                <div>
                  <b>{m.name}</b> <small className="muted">{m.version}</small>
                  {m.compatible === false && <em className="bad"> may not support {inst.mc_version} ({m.mc_constraint})</em>}
                  {m.compatible === true && <em className="ok"> compatible</em>}
                  <p className="muted small">{m.description || m.file}</p>
                </div>
                <div className="inline">
                  <Toggle on={m.enabled} onChange={async () => { try { await api.toggleMod(inst.id, m.file); loadMods(); } catch (e) { toast(errText(e), "err"); } }} />
                  <button className="btn danger sm" onClick={async () => { if (confirm(`Delete ${m.file}?`)) { try { await api.removeMod(inst.id, m.file); loadMods(); refresh(); } catch (e) { toast(errText(e), "err"); } } }}>Remove</button>
                </div>
              </div>
            ))}
          </Card>
        </>
      )}
    </>
  );
}
