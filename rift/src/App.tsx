import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { api, DownloadItem, Store } from "./api";
import { AppCtx, Page } from "./ctx";
import { Icon, Logo } from "./components/ui";
import Home from "./pages/Home";
import Games from "./pages/Games";
import GamePage from "./pages/GamePage";
import Optimizer from "./pages/Optimizer";
import Mods from "./pages/Mods";
import Downloads from "./pages/Downloads";
import Settings from "./pages/Settings";
import About from "./pages/About";

const NAV: { id: Page; label: string }[] = [
  { id: "home", label: "Home" },
  { id: "games", label: "My Games" },
  { id: "optimizer", label: "Optimizer" },
  { id: "mods", label: "Mods" },
  { id: "downloads", label: "Downloads" },
  { id: "settings", label: "Settings" },
  { id: "about", label: "About Rift" },
];

function hexToRgb(h: string) {
  const m = /^#?([0-9a-f]{6})$/i.exec(h.trim());
  if (!m) return "135,59,255";
  const n = parseInt(m[1], 16);
  return `${(n >> 16) & 255},${(n >> 8) & 255},${n & 255}`;
}

export default function App() {
  const [store, setStore] = useState<Store | null>(null);
  const [page, setPage] = useState<Page>("home");
  const [gameKey, setGameKey] = useState("");
  const [toasts, setToasts] = useState<{ id: number; msg: string; kind: string }[]>([]);
  const [fatal, setFatal] = useState("");
  const storeRef = useRef<Store | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await api.state();
      storeRef.current = s;
      setStore(s);
    } catch (e) {
      setFatal(String(e));
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  // apply accent / theme from saved settings
  useEffect(() => {
    if (!store) return;
    const r = document.documentElement.style;
    r.setProperty("--accent", store.settings.accent);
    r.setProperty("--accent-rgb", hexToRgb(store.settings.accent));
    document.documentElement.dataset.theme = store.settings.theme;
  }, [store?.settings.accent, store?.settings.theme]);

  const toast = useCallback((msg: string, kind: "ok" | "err" = "ok") => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t, { id, msg, kind }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), kind === "err" ? 9000 : 4000);
  }, []);

  const notify = useCallback((msg: string) => {
    if (!storeRef.current?.settings.notifications) return;
    try {
      if (Notification.permission === "granted") new Notification("Rift", { body: msg });
      else if (Notification.permission !== "denied") Notification.requestPermission().then((p) => p === "granted" && new Notification("Rift", { body: msg }));
    } catch { /* notifications unavailable */ }
  }, []);

  // download events -> refresh + notification
  useEffect(() => {
    const un = listen<DownloadItem>("download-update", (ev) => {
      if (ev.payload.status === "done") { notify(`${ev.payload.name} installed`); refresh(); }
      if (ev.payload.status === "failed") notify(`Download failed: ${ev.payload.name}`);
    });
    return () => { un.then((f) => f()); };
  }, [notify, refresh]);

  const go = useCallback((p: Page, key?: string) => {
    if (key !== undefined) setGameKey(key);
    setPage(p);
  }, []);
  const openGame = useCallback((key: string) => { setGameKey(key); setPage("game"); }, []);

  const win = getCurrentWindow();

  if (fatal) return <div className="fatal">Rift could not start: {fatal}</div>;
  if (!store) return <div className="fatal muted">Loading…</div>;

  const active = page === "game" ? "games" : page;
  return (
    <AppCtx.Provider value={{ store, refresh, go, openGame, toast, notify }}>
      <div className="app">
        <div className="titlebar" data-tauri-drag-region>
          <div className="brand" data-tauri-drag-region><Logo size={22} /><b>Rift</b></div>
          <div className="wc">
            <button onClick={() => win.minimize()} aria-label="Minimize">&#8211;</button>
            <button onClick={() => win.toggleMaximize()} aria-label="Maximize">&#9633;</button>
            <button className="x" onClick={() => win.close()} aria-label="Close">&#10005;</button>
          </div>
        </div>
        <aside className="side">
          <nav>
            {NAV.map((n) => (
              <button key={n.id} className={"nav" + (active === n.id ? " act" : "")} onClick={() => go(n.id)}>
                <Icon name={n.id === "games" ? "games" : n.id === "about" ? "about" : n.id} />
                {n.label}
              </button>
            ))}
          </nav>
          <div className="side-foot"><Logo size={44} /><b>Rift</b><span className="muted">Optimize. Launch. Enjoy.</span></div>
        </aside>
        <main className="main" key={page + gameKey}>
          {page === "home" && <Home />}
          {page === "games" && <Games />}
          {page === "game" && <GamePage gameKey={gameKey} />}
          {page === "optimizer" && <Optimizer />}
          {page === "mods" && <Mods />}
          {page === "downloads" && <Downloads />}
          {page === "settings" && <Settings />}
          {page === "about" && <About />}
        </main>
        <div className="toasts">
          {toasts.map((t) => <div key={t.id} className={"toast " + t.kind}>{t.msg}</div>)}
        </div>
      </div>
    </AppCtx.Provider>
  );
}
