import { useCallback, useEffect, useState } from "react";
import { api, Compat, errText, LocalMod, ModrinthProject } from "../api";
import { useApp } from "../ctx";
import { Card, Empty, Toggle } from "../components/ui";

export default function Mods() {
  const { store, refresh, toast, go } = useApp();
  const inst = store.instances.find((i) => i.id === store.selected_instance) ?? store.instances[0];
  const [featured, setFeatured] = useState<ModrinthProject[]>([]);
  const [compat, setCompat] = useState<Record<string, Compat>>({});
  const [q, setQ] = useState("");
  const [results, setResults] = useState<ModrinthProject[] | null>(null);
  const [installed, setInstalled] = useState<LocalMod[]>([]);
  const [loading, setLoading] = useState(false);

  const loadInstalled = useCallback(async () => {
    if (!inst) return;
    try { setInstalled((await api.listMods(inst.id)).mods); } catch { /* ignore */ }
  }, [inst?.id]);

  useEffect(() => { loadInstalled(); }, [loadInstalled, store.mod_records.length]);

  useEffect(() => {
    api.modFeatured().then(setFeatured).catch(() => toast("Could not reach Modrinth (are you offline?)", "err"));
  }, []);

  useEffect(() => {
    if (!inst || featured.length === 0) return;
    api.modCompat(featured.map((f) => f.slug), inst.mc_version).then((c) => setCompat(Object.fromEntries(c.map((x) => [x.slug, x])))).catch(() => {});
  }, [inst?.id, inst?.mc_version, featured.length]);

  const search = async () => {
    if (!q.trim()) return setResults(null);
    setLoading(true);
    try { setResults(await api.modSearch(q, inst?.mc_version ?? "")); } catch (e) { toast(errText(e), "err"); }
    setLoading(false);
  };

  const install = async (p: ModrinthProject) => {
    if (!inst) return;
    try { await api.installMod(inst.id, p.slug, p.title); toast(`Downloading ${p.title}…`); go("downloads"); } catch (e) { toast(errText(e), "err"); }
  };

  const isInstalled = (p: ModrinthProject) => store.mod_records.some((m) => m.instance_id === inst?.id && (m.slug === p.slug || m.project_id === (p.project_id ?? p.id))) || installed.some((m) => m.id === p.slug.replace(/-/g, "_") || m.id === p.slug);

  const modCard = (p: ModrinthProject) => {
    const c = compat[p.slug];
    const done = isInstalled(p);
    return (
      <div className="mod" key={p.slug}>
        {p.icon_url ? <img src={p.icon_url} alt="" /> : <div className="mod-ph">{p.title[0]}</div>}
        <div className="grow">
          <b>{p.title}</b>
          <p className="muted small">{p.description}</p>
          <p className="small">
            {p.downloads.toLocaleString()} downloads
            {inst && c && (c.compatible ? <em className="ok"> · works with Minecraft {inst.mc_version} (v{c.version})</em> : <em className="bad"> · no Fabric build for {inst.mc_version}</em>)}
            {c && c.requires.length > 0 && <span className="muted"> · requires {c.requires.length} other mod(s), installed automatically</span>}
          </p>
        </div>
        <button className="btn primary" disabled={!inst || done || (c && !c.compatible)} onClick={() => install(p)}>{done ? "Installed" : "Install"}</button>
      </div>
    );
  };

  return (
    <>
      <h1>Mods</h1>
      <Card title="Minecraft (Fabric)" sub="Mods come from Modrinth. Rift only installs files that match your Minecraft version and the Fabric loader, from Modrinth's own CDN, and verifies their checksum.">
        {!inst ? (
          <Empty>Create a Minecraft profile first. <button className="link" onClick={() => go("game", "minecraft-java")}>Open Minecraft →</button></Empty>
        ) : (
          <p>Installing into <b>{inst.name}</b> · Minecraft {inst.mc_version} · Fabric {inst.loader_version}. Change profile on the Minecraft page.</p>
        )}
        <div className="search small-search">
          <input placeholder="Search Modrinth for Fabric mods…" value={q} onChange={(e) => setQ(e.target.value)} onKeyDown={(e) => e.key === "Enter" && search()} />
          <button className="btn primary" onClick={search} disabled={loading}>{loading ? "Searching…" : "Search"}</button>
        </div>
      </Card>

      {results && <Card title={`Search results (${results.length})`}>{results.length === 0 ? <Empty>No mods found for this Minecraft version.</Empty> : results.map(modCard)}</Card>}

      <Card title="Recommended performance mods" sub="Sodium, Lithium, FerriteCore, ImmediatelyFast, Entity Culling, ModernFix, Iris, More Culling (+ Fabric API)">
        {featured.length === 0 ? <Empty>Loading from Modrinth…</Empty> : featured.map(modCard)}
      </Card>

      <Card title="Installed in this profile" sub={inst ? inst.name : ""}>
        {installed.length === 0 ? <Empty>No mods installed.</Empty> : installed.map((m) => (
          <div className="row" key={m.file}>
            <div><b>{m.name}</b> <small className="muted">{m.version}</small>
              {m.compatible === false && <em className="bad"> may not support {inst?.mc_version}</em>}
              {m.depends.filter((d) => !["minecraft", "java", "fabricloader"].includes(d)).length > 0 && <p className="muted small">Depends on: {m.depends.filter((d) => !["minecraft", "java", "fabricloader"].includes(d)).join(", ")}</p>}
            </div>
            <div className="inline">
              <Toggle on={m.enabled} onChange={async () => { if (!inst) return; try { await api.toggleMod(inst.id, m.file); loadInstalled(); } catch (e) { toast(errText(e), "err"); } }} />
              <button className="btn danger sm" onClick={async () => { if (!inst || !confirm(`Delete ${m.file}?`)) return; try { await api.removeMod(inst.id, m.file); loadInstalled(); refresh(); } catch (e) { toast(errText(e), "err"); } }}>Uninstall</button>
            </div>
          </div>
        ))}
      </Card>

      <Card title="Other games">
        <p className="muted">Fortnite, VALORANT, CS2 and Roblox use anti-cheat and do not support mods, so Rift offers none. GTA V single-player mods and Terraria's tModLoader are managed by their own tools, and Rift does not install files for them.</p>
      </Card>
    </>
  );
}
