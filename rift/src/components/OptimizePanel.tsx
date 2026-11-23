import { useState } from "react";
import { api, errText, Game, Opts } from "../api";
import { CatalogGame } from "../catalog";
import { useApp } from "../ctx";
import { Card, Toggle } from "./ui";

/** Game-specific optimisation controls. Only options that Rift can really apply for this game are shown. */
export default function OptimizePanel({ cg, game }: { cg: CatalogGame; game?: Game }) {
  const { toast, refresh } = useApp();
  const [o, setO] = useState<Opts>({ particles: false, graphics: false, ram: false, power_plan: false, game_mode: false });
  const [fps, setFps] = useState(144);
  const [useFps, setUseFps] = useState(false);
  const [vsync, setVsync] = useState(false);
  const [useVsync, setUseVsync] = useState(false);
  const [fsMode, setFsMode] = useState(-1);
  const [args, setArgs] = useState(game?.args ?? "");
  const [report, setReport] = useState<{ changes: string[]; skipped: string[] } | null>(null);
  const [busy, setBusy] = useState(false);

  const mc = cg.tools.includes("minecraft");
  const fn = cg.tools.includes("fortnite-settings");
  const la = cg.tools.includes("launch-args");

  const apply = async () => {
    const opts: Opts = { ...o };
    if (useFps) opts.fps = fps;
    if (useVsync) opts.vsync = vsync;
    if (fsMode >= 0) opts.fullscreen_mode = fsMode;
    if (la && game && args !== game.args) opts.launch_args = args;
    setBusy(true);
    try {
      const r = await api.optimize(game?.id ?? cg.key, opts);
      setReport(r);
      toast(r.changes.length ? `${r.changes.length} change(s) applied - backup saved` : "No changes were made", r.changes.length ? "ok" : "err");
      await refresh();
    } catch (e) {
      toast(errText(e), "err");
    }
    setBusy(false);
  };

  const row = (label: string, desc: string, on: boolean, set: (v: boolean) => void) => (
    <div className="row">
      <div><b>{label}</b><p className="muted">{desc}</p></div>
      <Toggle on={on} onChange={set} />
    </div>
  );

  return (
    <Card title="Optimization" sub="Only changes Rift can really make for this game. Every change is backed up and reversible.">
      {mc && (
        <>
          {row("Allocate more RAM", "Sets the selected profile's RAM to about half of your system memory (2-8 GB).", !!o.ram, (v) => setO({ ...o, ram: v }))}
          {row("Disable particles", "Writes particles:2 (Minimal) to options.txt.", !!o.particles, (v) => setO({ ...o, particles: v }))}
          {row("Optimize graphics", "Fast graphics, entity shadows off, lower biome blend and mipmaps.", !!o.graphics, (v) => setO({ ...o, graphics: v }))}
          <div className="row">
            <div><b>Max FPS</b><p className="muted">Writes maxFps to options.txt.</p></div>
            <div className="inline"><input type="number" min={10} max={260} value={fps} onChange={(e) => setFps(+e.target.value)} /><Toggle on={useFps} onChange={setUseFps} /></div>
          </div>
        </>
      )}
      {fn && (
        <>
          <div className="row">
            <div><b>FPS limit</b><p className="muted">GameUserSettings.ini - FrameRateLimit. Close Fortnite first.</p></div>
            <div className="inline"><input type="number" min={30} max={360} value={fps} onChange={(e) => setFps(+e.target.value)} /><Toggle on={useFps} onChange={setUseFps} /></div>
          </div>
          <div className="row">
            <div><b>VSync</b><p className="muted">Off lowers latency; on removes tearing.</p></div>
            <div className="inline"><Toggle on={vsync} onChange={setVsync} /><span className="muted">apply</span><Toggle on={useVsync} onChange={setUseVsync} /></div>
          </div>
          <div className="row">
            <div><b>Window mode</b><p className="muted">Fullscreen, windowed fullscreen or windowed.</p></div>
            <select value={fsMode} onChange={(e) => setFsMode(+e.target.value)}>
              <option value={-1}>Leave unchanged</option>
              <option value={0}>Fullscreen</option>
              <option value={1}>Windowed Fullscreen</option>
              <option value={2}>Windowed</option>
            </select>
          </div>
        </>
      )}
      {la && (
        <div className="row col">
          <div><b>Launch options</b><p className="muted">{game?.kind === "steam" ? "Passed to Steam when Rift launches the game (steam://run)." : "Passed to the game when Rift launches it."} Example for CS2: <code>-novid -high</code></p></div>
          <input value={args} disabled={!game} placeholder={game ? "e.g. -novid" : "Game not detected yet"} onChange={(e) => setArgs(e.target.value)} />
        </div>
      )}
      {row("Windows High performance power plan", "Switches the active power plan. The previous plan is restored by Undo.", !!o.power_plan, (v) => setO({ ...o, power_plan: v }))}
      {row("Windows Game Mode", "Turns on Game Mode for your user account.", !!o.game_mode, (v) => setO({ ...o, game_mode: v }))}
      <div className="actions">
        <button className="btn primary" disabled={busy} onClick={apply}>{busy ? "Applying…" : "Optimize selected game"}</button>
      </div>
      {report && (
        <div className="report">
          {report.changes.map((c) => <div key={c} className="ok-line">✔ {c}</div>)}
          {report.skipped.map((c) => <div key={c} className="skip-line">• {c}</div>)}
          {!report.changes.length && !report.skipped.length && <div className="muted">Nothing was selected.</div>}
        </div>
      )}
    </Card>
  );
}
