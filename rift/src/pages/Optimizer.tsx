import { useCallback, useEffect, useState } from "react";
import { api, errText, Proc, SysLive, SysStatic } from "../api";
import { catalogFor, CATALOG, fmtTime, GENERIC } from "../catalog";
import { useApp } from "../ctx";
import { Card, Empty, Progress } from "../components/ui";
import OptimizePanel from "../components/OptimizePanel";

export default function Optimizer() {
  const { store, refresh, toast } = useApp();
  const [live, setLive] = useState<SysLive | null>(null);
  const [stat, setStat] = useState<SysStatic | null>(null);
  const [procs, setProcs] = useState<Proc[]>([]);
  const [sel, setSel] = useState(store.games[0]?.id ?? "minecraft-java");
  const [log, setLog] = useState<string[]>([]);

  const loadProcs = useCallback(() => api.processes().then(setProcs).catch(() => {}), []);
  useEffect(() => {
    let alive = true;
    const tick = () => api.sysLive().then((l) => alive && setLive(l)).catch(() => {});
    tick(); loadProcs();
    const t = setInterval(tick, 2000);
    api.sysStatic().then((s) => alive && setStat(s)).catch(() => {});
    return () => { alive = false; clearInterval(t); };
  }, [loadProcs]);

  const game = store.games.find((g) => g.id === sel);
  const cg = catalogFor(game?.catalog_key || sel) ?? (game ? { ...GENERIC, name: game.name } : undefined);
  const options = [
    ...store.games.map((g) => ({ id: g.id, name: g.name })),
    ...CATALOG.filter((c) => !store.games.some((g) => g.catalog_key === c.key)).map((c) => ({ id: c.key, name: `${c.name} (not detected)` })),
  ];
  const ramPct = live ? (live.ram_used_mb / Math.max(1, live.ram_total_mb)) * 100 : 0;
  const active = store.backups.filter((b) => !b.restored).sort((a, b) => b.ts - a.ts);
  const history = store.backups.filter((b) => b.restored).sort((a, b) => b.ts - a.ts).slice(0, 5);

  return (
    <>
      <h1>Optimizer</h1>
      <div className="two">
        <Card title="Your PC" sub="Live">
          {live ? (
            <>
              <div className="meter"><span>CPU {live.cpu_usage.toFixed(0)}%</span><Progress value={live.cpu_usage} /></div>
              <p className="muted small">{live.cpu_name} · {live.cores} threads</p>
              <div className="meter"><span>RAM {(live.ram_used_mb / 1024).toFixed(1)} / {(live.ram_total_mb / 1024).toFixed(1)} GB</span><Progress value={ramPct} /></div>
            </>
          ) : <Empty>Reading…</Empty>}
          <div className="kv">
            <div><span>GPU</span><b>{stat ? (stat.gpus.join(" | ") || "Not reported") : "Reading…"}</b></div>
            <div><span>Power plan</span><b>{stat?.power_plan || "…"}</b></div>
            <div><span>Game Mode</span><b>{stat?.game_mode == null ? "Not set" : stat.game_mode ? "On" : "Off"}</b></div>
          </div>
          <p className="muted small">FPS counters need a frame-capture tool (such as PresentMon) and are not built into Rift yet, so none is shown rather than a fake number.</p>
        </Card>
        <Card title="Windows gaming recommendations" sub="Advice - Rift does not apply these automatically">
          <ul className="tips">
            <li>Use the High performance power plan while gaming (button below can apply it, reversibly).</li>
            <li>Keep Windows Game Mode on.</li>
            <li>Update your GPU driver from NVIDIA, AMD or Intel directly.</li>
            <li>Install games on an SSD where possible.</li>
            <li>Leave Windows Security / Defender and anti-cheat services running - Rift will never disable them.</li>
            <li>Close browsers, launchers and overlays you do not need (see the list below).</li>
          </ul>
        </Card>
      </div>

      <Card title="Optimize a game" right={
        <select value={sel} onChange={(e) => setSel(e.target.value)}>{options.map((o) => <option key={o.id} value={o.id}>{o.name}</option>)}</select>
      }>
        <p className="muted small">Choose the game, switch on what you want, then press <b>Optimize selected game</b>.</p>
      </Card>
      {cg && <OptimizePanel key={sel} cg={cg} game={game} />}

      <Card title="Undo" sub="Every change Rift made is listed here with its backup.">
        {active.length === 0 ? <Empty>Nothing to undo - Rift has no active changes.</Empty> : active.map((b) => (
          <div className="row" key={b.id}>
            <div><b>{b.label}</b><p className="muted small">{fmtTime(b.ts)} · {b.undo.length} item(s)</p></div>
            <button className="btn" onClick={async () => { try { const l = await api.restore(b.id); setLog(l); toast("Previous settings restored"); refresh(); api.sysStatic().then(setStat); } catch (e) { toast(errText(e), "err"); } }}>Restore previous settings</button>
          </div>
        ))}
        <div className="actions">
          <button className="btn danger" disabled={!active.length} onClick={async () => {
            if (!confirm("Undo every change Rift has made?")) return;
            try { setLog(await api.resetOpt()); toast("All optimizations reset"); refresh(); api.sysStatic().then(setStat); } catch (e) { toast(errText(e), "err"); }
          }}>Reset optimization</button>
        </div>
        {log.length > 0 && <div className="report">{log.map((l, i) => <div key={i} className={l.startsWith("FAILED") ? "bad-line" : "ok-line"}>{l}</div>)}</div>}
        {history.length > 0 && <p className="muted small">Already restored: {history.map((h) => h.label).join("; ")}</p>}
      </Card>

      <Card title="Startup applications" sub="What Windows starts at sign-in (read only - disable them in Task Manager > Startup)">
        {!stat ? <Empty>Reading…</Empty> : stat.startup.length === 0 ? <Empty>None reported.</Empty> : stat.startup.map((s, i) => (
          <div className="row" key={i}><div><b>{s.name}</b><p className="muted small">{s.command}</p></div></div>
        ))}
      </Card>

      <Card title="Background applications" sub="Biggest memory users. You choose what to close; Windows and security processes are protected." right={<button className="btn" onClick={loadProcs}>Refresh</button>}>
        {procs.map((p) => (
          <div className="row" key={p.pid}>
            <div><b>{p.name}</b> <small className="muted">PID {p.pid} · {p.mem_mb} MB</small></div>
            <button className="btn sm" disabled={p.protected} title={p.protected ? "Protected" : "Close this program"} onClick={async () => {
              if (!confirm(`Close ${p.name}? Unsaved work in it will be lost.`)) return;
              try { toast(await api.endProcess(p.pid)); loadProcs(); } catch (e) { toast(errText(e), "err"); }
            }}>{p.protected ? "Protected" : "Close"}</button>
          </div>
        ))}
      </Card>
    </>
  );
}
