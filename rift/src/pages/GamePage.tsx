import { useState } from "react";
import { api, errText } from "../api";
import { catalogFor, CatalogGame, fmtTime, GENERIC } from "../catalog";
import { useApp } from "../ctx";
import { Card } from "../components/ui";
import OptimizePanel from "../components/OptimizePanel";
import MinecraftPanel from "../components/MinecraftPanel";
import { getCurrentWindow } from "@tauri-apps/api/window";

export default function GamePage({ gameKey }: { gameKey: string }) {
  const { store, refresh, toast, go } = useApp();
  const [busy, setBusy] = useState(false);
  const game = store.games.find((g) => g.id === gameKey || g.catalog_key === gameKey);
  const cg: CatalogGame | undefined = catalogFor(game?.catalog_key || gameKey) ?? (game ? { ...GENERIC, name: game.name, letter: game.name[0]?.toUpperCase() ?? "G" } : undefined);
  const [a, b] = (cg?.hue ?? "#873bff,#2a1a4a").split(",");
  const name = game?.name ?? cg?.name ?? gameKey;
  const isMcJava = cg?.key === "minecraft-java";

  const play = async () => {
    if (!game) return;
    setBusy(true);
    try {
      await api.launch(game.id);
      toast(`${game.name} is starting…`);
      await refresh();
      const lb = store.settings.launch_behavior;
      if (lb === "minimize") await getCurrentWindow().minimize();
      if (lb === "close") await getCurrentWindow().close();
    } catch (e) { toast(errText(e), "err"); }
    setBusy(false);
  };

  return (
    <>
      <div className="game-hero" style={{ background: `linear-gradient(120deg, ${a}55, ${b}22 60%, transparent)` }}>
        <div className="tile-art xl" style={{ background: `linear-gradient(135deg, ${a}, ${b})` }}><span>{cg?.letter ?? name[0]}</span></div>
        <div className="grow">
          <h1>{name} {isMcJava && <span className="pill">Fabric</span>}</h1>
          {game ? (
            <>
              <p className="muted">{game.exe || game.path || game.uri}</p>
              <p className="muted small">Launch via: {cg?.launch ?? game.kind} · Last played: {fmtTime(game.last_played)}</p>
            </>
          ) : <p className="muted">Not detected on this PC yet.</p>}
        </div>
        {game && !isMcJava && <button className="btn primary big" disabled={busy} onClick={play}>▶ Play</button>}
        {game && isMcJava && <button className="btn" disabled={busy} onClick={play}>Open Minecraft Launcher</button>}
      </div>

      {!game && (
        <Card title="Not in your library">
          <p className="muted">Rift only shows a Play button for games it has actually found. Scan your drives or add the game's .exe manually.</p>
          <div className="actions"><button className="btn primary" onClick={() => go("games")}>Go to My Games</button></div>
        </Card>
      )}

      {isMcJava && <MinecraftPanel />}

      {game && cg && !isMcJava && <OptimizePanel cg={cg} game={game} />}

      {cg && (
        <Card title="Recommendations" sub="Advice only - nothing here changes your system.">
          <ul className="tips">{cg.tips.map((t) => <li key={t}>{t}</li>)}</ul>
          <p className="muted small">Mods: {cg.mods}</p>
        </Card>
      )}

      {game && (
        <Card title="Library">
          <div className="actions">
            <button className="btn danger" onClick={async () => {
              if (!confirm(`Remove ${game.name} from Rift?\nThe game itself is NOT uninstalled.`)) return;
              await api.removeGame(game.id); await refresh(); go("games");
            }}>Remove from Rift</button>
          </div>
        </Card>
      )}
    </>
  );
}
