import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errText } from "../api";
import { catalogFor, CATALOG, GENERIC } from "../catalog";
import { useApp } from "../ctx";
import { Card, Empty, GameTile } from "../components/ui";

export default function Games() {
  const { store, refresh, toast, openGame } = useApp();
  const [drives, setDrives] = useState<string[]>([]);
  const [pickedDrives, setPickedDrives] = useState<string[]>([]);
  const [folders, setFolders] = useState<string[]>(store.settings.scan_folders);
  const [busy, setBusy] = useState(false);
  const [last, setLast] = useState("");

  useEffect(() => { api.drives().then(setDrives).catch(() => {}); }, []);

  const addFolder = async () => {
    const f = await open({ directory: true, multiple: false });
    if (typeof f === "string" && !folders.includes(f)) {
      const next = [...folders, f];
      setFolders(next);
      await api.saveSettings({ ...store.settings, scan_folders: next });
      refresh();
    }
  };
  const removeFolder = async (f: string) => {
    const next = folders.filter((x) => x !== f);
    setFolders(next);
    await api.saveSettings({ ...store.settings, scan_folders: next });
    refresh();
  };
  const scan = async () => {
    setBusy(true); setLast("");
    try {
      const r = await api.scan(folders, pickedDrives);
      setLast(r.added.length ? `Added: ${r.added.join(", ")}` : `Scan finished (${r.found} candidate(s) examined) - nothing new to add.`);
      await refresh();
    } catch (e) { toast(errText(e), "err"); }
    setBusy(false);
  };
  const addExe = async () => {
    const f = await open({ multiple: false, filters: [{ name: "Executable", extensions: ["exe"] }] });
    if (typeof f !== "string") return;
    try { const g = await api.addGame(f); await refresh(); toast(`Added ${g.name}`); } catch (e) { toast(errText(e), "err"); }
  };
  const addDir = async () => {
    const f = await open({ directory: true, multiple: false });
    if (typeof f !== "string") return;
    try { const g = await api.addGame(f); await refresh(); toast(`Added ${g.name}`); } catch (e) { toast(errText(e), "err"); }
  };

  const keys = new Set(store.games.map((g) => g.catalog_key).filter(Boolean));

  return (
    <>
      <h1>My Games</h1>
      <Card title="Find games" sub="Steam, Epic, Minecraft, Roblox and VALORANT are always checked. Choose extra drives or folders below - only well-known game folders on a drive are looked at, never system locations.">
        <div className="chips">
          {drives.map((d) => (
            <label key={d} className={"chip" + (pickedDrives.includes(d) ? " act" : "")}>
              <input type="checkbox" checked={pickedDrives.includes(d)} onChange={(e) => setPickedDrives(e.target.checked ? [...pickedDrives, d] : pickedDrives.filter((x) => x !== d))} /> Drive {d}
            </label>
          ))}
        </div>
        <div className="folders">
          {folders.map((f) => <div key={f} className="row"><code>{f}</code><button className="btn sm" onClick={() => removeFolder(f)}>Remove</button></div>)}
        </div>
        <div className="actions">
          <button className="btn" onClick={addFolder}>+ Scan folder…</button>
          <button className="btn" onClick={addDir}>+ Add game folder…</button>
          <button className="btn" onClick={addExe}>+ Add game .exe…</button>
          <button className="btn primary" disabled={busy} onClick={scan}>{busy ? "Scanning…" : "Scan now"}</button>
        </div>
        {last && <p className="ok-line">{last}</p>}
      </Card>

      <h2>Library ({store.games.length})</h2>
      {store.games.length === 0 ? <Empty>Your library is empty. Scan or add a game above.</Empty> : (
        <div className="grid">
          {store.games.map((g) => {
            const cg = catalogFor(g.catalog_key) ?? GENERIC;
            return <GameTile key={g.id} g={{ ...cg, name: g.name, letter: cg === GENERIC ? g.name[0].toUpperCase() : cg.letter }} installed onClick={() => openGame(g.id)} />;
          })}
        </div>
      )}

      <h2>Supported games not detected</h2>
      <div className="grid">
        {CATALOG.filter((c) => !keys.has(c.key)).map((c) => <GameTile key={c.key} g={c} installed={false} onClick={() => openGame(c.key)} />)}
      </div>
    </>
  );
}
