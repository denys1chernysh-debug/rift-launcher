import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, DownloadItem, errText } from "../api";
import { fmtBytes, fmtTime } from "../catalog";
import { useApp } from "../ctx";
import { Card, Empty, Progress } from "../components/ui";

export default function Downloads() {
  const { store, toast } = useApp();
  const [items, setItems] = useState<DownloadItem[]>([]);

  useEffect(() => {
    api.downloads().then(setItems).catch(() => {});
    const un = listen<DownloadItem>("download-update", (ev) =>
      setItems((cur) => (cur.some((c) => c.id === ev.payload.id) ? cur.map((c) => (c.id === ev.payload.id ? ev.payload : c)) : [...cur, ev.payload]))
    );
    return () => { un.then((f) => f()); };
  }, []);

  const current = items.filter((d) => d.status === "queued" || d.status === "downloading");
  const failed = items.filter((d) => d.status === "failed" || d.status === "cancelled");
  const done = items.filter((d) => d.status === "done");
  const records = [...store.mod_records].sort((a, b) => b.ts - a.ts);

  const retry = async (id: string) => { try { await api.retry(id); } catch (e) { toast(errText(e), "err"); } };

  return (
    <>
      <h1>Downloads</h1>
      <Card title="Current downloads">
        {current.length === 0 ? <Empty>No active downloads.</Empty> : current.map((d) => (
          <div className="dl" key={d.id}>
            <div className="row"><b>{d.name}</b>
              <button className="btn sm danger" onClick={() => api.cancel(d.id)}>Cancel</button></div>
            <Progress value={d.total ? (d.received / d.total) * 100 : 0} />
            <p className="muted small">{d.status === "queued" ? "Waiting…" : `${fmtBytes(d.received)} / ${d.total ? fmtBytes(d.total) : "?"} · ${fmtBytes(d.speed)}/s`}</p>
          </div>
        ))}
      </Card>
      <Card title="Failed or cancelled" right={items.length > 0 && <button className="btn sm" onClick={async () => { await api.clearDownloads(); setItems(await api.downloads()); }}>Clear finished</button>}>
        {failed.length === 0 ? <Empty>Nothing failed.</Empty> : failed.map((d) => (
          <div className="row" key={d.id}>
            <div><b>{d.name}</b><p className="bad-line small">{d.status === "cancelled" ? "Cancelled" : d.error}</p></div>
            <button className="btn sm" onClick={() => retry(d.id)}>Retry</button>
          </div>
        ))}
      </Card>
      <Card title="Finished this session">
        {done.length === 0 ? <Empty>None yet.</Empty> : done.map((d) => <div className="row" key={d.id}><b>{d.name}</b><span className="ok-line">{fmtBytes(d.total)}</span></div>)}
      </Card>
      <Card title="Installed mods (all profiles)">
        {records.length === 0 ? <Empty>No mods installed through Rift yet.</Empty> : records.map((m) => (
          <div className="row" key={m.instance_id + m.file}>
            <div><b>{m.title}</b> <small className="muted">{m.version}</small><p className="muted small">{store.instances.find((i) => i.id === m.instance_id)?.name ?? m.instance_id} · {fmtTime(m.ts)}</p></div>
          </div>
        ))}
      </Card>
    </>
  );
}
