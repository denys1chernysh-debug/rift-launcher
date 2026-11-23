import { useEffect, useMemo, useState } from "react";
import { api, errText, SysLive, SysStatic } from "../api";
import { CATALOG, catalogFor, fmtTime } from "../catalog";
import { useApp } from "../ctx";
import { Card, Empty, GameTile, Icon, Logo } from "../components/ui";
import { open } from "@tauri-apps/plugin-dialog";

export default function Home() {
  const { store, refresh, go, openGame, toast } = useApp();
  const [q, setQ] = useState("");
  const [live, setLive] = useState<SysLive | null>(null);
  const [stat, setStat] = useState<SysStatic | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let alive = true;
    const tick = () => api.sysLive().then((l) => alive && setLive(l)).catch(() => {});
    tick();
    const t = setInterval(tick, 2500);
    api.sysStatic().then((s) => alive && setStat(s)).catch(() => {});
    return () => { alive = false; clearInterval(t); };
  }, []);

  const installedKeys = new Set(store.games.map((g) => g.catalog_key).filter(Boolean));
  const ql = q.trim().toLowerCase();
  const results = useMemo(() => {
    if (!ql) return [];
    const lib = store.games.filter((g) => g.name.toLowerCase().includes(ql)).map((g) => ({ id: g.id, name: g.name, installed: true }));
    const cat = CATALOG.filter((c) => c.name.toLowerCase().includes(ql) && !installedKeys.has(c.key)).map((c) => ({ id: c.key, name: c.name, installed: false }));
    return [...lib, ...cat];
  }, [ql, store.games]);

  const recent = [...store.games].filter((g) => g.last_played > 0).sort((a, b) => b.last_played - a.last_played).slice(0, 5);
  const mods = [...store.mod_records].sort((a, b) => b.ts - a.ts).slice(0, 4);
  const mc = store.games.find((g) => g.catalog_key === "minecraft-java");

  const scan = async () => {
    setBusy(true);
    try {
      const r = await api.scan(store.settings.scan_folders, []);
      toast(r.added.length ? `Added ${r.added.length} game(s): ${r.added.slice(0, 4).join(", ")}` : "Scan finished - no new games found");
      await refresh();
    } catch (e) { toast(errText(e), "err"); }
    setBusy(false);
  };
  const add = async () => {
    const f = await open({ multiple: false, filters: [{ name: "Game executable", extensions: ["exe"] }] });
    if (typeof f !== "string") return;
    try { const g = await api.addGame(f); await refresh(); toast(`Added ${g.name}`); } catch (e) { toast(errText(e), "err"); }
  };

  const ramPct = live ? Math.round((live.ram_used_mb / Math.max(1, live.ram_total_mb)) * 100) : 0;

  return (
    <>
      <div className="hero">
        <Logo size={84} />
        <div className="grow">
          <h1>Rift</h1>
          <p className="tag">Optimize. Launch. Enjoy.</p>
        </div>
        <div className="status-card">
          <Icon name="optimizer" />
          <div>
            <b>{store.games.length ? `${store.games.length} game${store.games.length > 1 ? "s" : ""} in your library` : "No games yet"}</b>
            <p className="muted">{store.backups.filter((b) => !b.restored).length} active optimization(s)</p>
          </div>
        </div>
      </div>

      <div className="search">
        <Icon name="search" />
        <input placeholder="Search for games…" value={q} onChange={(e) => setQ(e.target.value)} />
        <button className="btn" onClick={add}>+ Add Game</button>
        <button className="btn primary" disabled={busy} onClick={scan}>{busy ? "Scanning…" : "Scan"}</button>
      </div>
      {ql && (
        <Card title={`Results for "${q}"`}>
          {results.length === 0 ? <Empty>No match in your library or the supported games list.</Empty> : (
            <div className="chips">{results.map((r) => <button key={r.id} className="chip" onClick={() => openGame(r.id)}>{r.name} <small>{r.installed ? "installed" : "not detected"}</small></button>)}</div>
          )}
        </Card>
      )}

      {recent.length > 0 && (
        <>
          <h2>Recently played</h2>
          <div className="grid">
            {recent.map((g) => {
              const cg = catalogFor(g.catalog_key) ?? { key: g.id, name: g.name, hue: "#873bff,#2a1a4a", letter: g.name[0].toUpperCase() } as any;
              return <GameTile key={g.id} g={{ ...cg, name: g.name }} installed onClick={() => openGame(g.id)} />;
            })}
          </div>
        </>
      )}

      <div className="h-row"><h2>Popular games</h2><button className="link" onClick={() => go("games")}>View all →</button></div>
      <div className="grid">
        {CATALOG.map((c) => <GameTile key={c.key} g={c} installed={installedKeys.has(c.key)} onClick={() => openGame(c.key)} />)}
      </div>

      <div className="h-row"><h2>Detected games</h2></div>
      {store.games.length === 0 ? (
        <Empty>Nothing detected yet. Press <b>Scan</b> to look at Steam, Epic, Minecraft and your chosen folders - or add a game manually.</Empty>
      ) : (
        <div className="list">
          {store.games.map((g) => (
            <button key={g.id} className="list-item" onClick={() => openGame(g.id)}>
              <b>{g.name}</b><span className="muted">{g.kind} · {g.exe || g.path}</span>
            </button>
          ))}
        </div>
      )}

      <div className="two">
        <Card title="Quick optimization" sub="Real, reversible Windows tweaks">
          <div className="row"><div><b>Power plan</b><p className="muted">{stat ? `Currently: ${stat.power_plan || "unknown"}` : "Reading…"}</p></div>
            <button className="btn" onClick={async () => { try { const r = await api.optimize("system", { power_plan: true }); toast(r.changes[0] ?? r.skipped[0] ?? "No change", r.changes.length ? "ok" : "err"); await refresh(); api.sysStatic().then(setStat); } catch (e) { toast(errText(e), "err"); } }}>High performance</button></div>
          <div className="row"><div><b>Windows Game Mode</b><p className="muted">{stat?.game_mode == null ? "Not set" : stat.game_mode ? "On" : "Off"}</p></div>
            <button className="btn" onClick={async () => { try { const r = await api.optimize("system", { game_mode: true }); toast(r.changes[0] ?? r.skipped[0] ?? "No change", r.changes.length ? "ok" : "err"); await refresh(); api.sysStatic().then(setStat); } catch (e) { toast(errText(e), "err"); } }}>Turn on</button></div>
          <div className="actions"><button className="link" onClick={() => go("optimizer")}>Open Optimizer →</button></div>
        </Card>
        <Card title="Gaming performance" sub="Live from your PC">
          {live ? (
            <div className="kv">
              <div><span>CPU</span><b>{live.cpu_name}</b> · {live.cores} threads · <b>{live.cpu_usage.toFixed(0)}%</b></div>
              <div><span>RAM</span><b>{(live.ram_used_mb / 1024).toFixed(1)} / {(live.ram_total_mb / 1024).toFixed(1)} GB</b> ({ramPct}%)</div>
              <div><span>GPU</span><b>{stat ? stat.gpus.join(" | ") || "Not reported" : "Reading…"}</b></div>
            </div>
          ) : <Empty>Reading…</Empty>}
        </Card>
      </div>

      <Card title="Recently installed mods" right={<button className="link" onClick={() => go("mods")}>Browse mods →</button>}>
        {mods.length === 0 ? <Empty>No mods installed through Rift yet.</Empty> : (
          <div className="chips">{mods.map((m) => <span key={m.file} className="chip">{m.title} <small>{fmtTime(m.ts)}</small></span>)}</div>
        )}
      </Card>
      {mc && <p className="muted small">Minecraft Java found at {mc.path}</p>}
    </>
  );
}
