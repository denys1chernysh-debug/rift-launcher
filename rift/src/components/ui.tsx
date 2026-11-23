import { ReactNode } from "react";
import { CatalogGame } from "../catalog";

export const Logo = ({ size = 28 }: { size?: number }) => (
  <img src="/logo.svg" width={size} height={size} alt="Rift" draggable={false} />
);

export const Toggle = ({ on, onChange, disabled }: { on: boolean; onChange: (v: boolean) => void; disabled?: boolean }) => (
  <button className={"toggle" + (on ? " on" : "")} disabled={disabled} onClick={() => onChange(!on)} aria-pressed={on}>
    <span />
  </button>
);

export const Card = ({ title, sub, right, children, className }: { title?: string; sub?: string; right?: ReactNode; children: ReactNode; className?: string }) => (
  <section className={"card " + (className ?? "")}>
    {(title || right) && (
      <header className="card-h">
        <div>
          {title && <h3>{title}</h3>}
          {sub && <p className="muted">{sub}</p>}
        </div>
        {right}
      </header>
    )}
    {children}
  </section>
);

export const GameTile = ({ g, installed, onClick, selected }: { g: CatalogGame; installed: boolean; onClick: () => void; selected?: boolean }) => {
  const [a, b] = g.hue.split(",");
  return (
    <button className={"tile" + (selected ? " sel" : "")} onClick={onClick}>
      <div className="tile-art" style={{ background: `linear-gradient(135deg, ${a}, ${b})` }}>
        <span>{g.letter}</span>
      </div>
      <div className="tile-name">{g.name}</div>
      <div className={"tile-badge" + (installed ? " ok" : "")}>{installed ? "Installed" : "Not detected"}</div>
    </button>
  );
};

export const Progress = ({ value }: { value: number }) => (
  <div className="bar"><i style={{ width: `${Math.min(100, Math.max(0, value))}%` }} /></div>
);

export const Empty = ({ children }: { children: ReactNode }) => <div className="empty">{children}</div>;

export const Icon = ({ name }: { name: string }) => {
  const p: Record<string, string> = {
    home: "M3 11l9-8 9 8v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1z",
    games: "M6 8h12a4 4 0 0 1 4 4v3a3 3 0 0 1-5.2 2L15 15H9l-1.8 2A3 3 0 0 1 2 15v-3a4 4 0 0 1 4-4zm2 2v1H7v2h1v1h2v-1h1v-2h-1v-1zm8 1a1 1 0 1 0 0 2 1 1 0 0 0 0-2z",
    optimizer: "M13 2L4 14h6l-1 8 9-12h-6z",
    mods: "M10 3a2 2 0 0 1 4 0v2h4a1 1 0 0 1 1 1v4h2a2 2 0 0 1 0 4h-2v4a1 1 0 0 1-1 1h-4v-2a2 2 0 0 0-4 0v2H6a1 1 0 0 1-1-1v-4H3a2 2 0 0 1 0-4h2V6a1 1 0 0 1 1-1h4z",
    downloads: "M12 3v12m0 0l-5-5m5 5l5-5M4 20h16",
    settings: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zm8 4l2-1-2-4-2 .5-1.5-1.5.5-2-4-2-1 2h-2l-1-2-4 2 .5 2L5 7.5 3 7l-2 4 2 1v2l-2 1 2 4 2-.5L6.5 20l-.5 2 4 2 1-2h2l1 2 4-2-.5-2L19 18.5l2 .5 2-4-2-1z",
    about: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm1 15h-2v-6h2zm0-8h-2V7h2z",
    play: "M6 4l14 8-14 8z",
    plus: "M12 5v14M5 12h14",
    search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zm9 16l-4-4",
  };
  const stroke = ["downloads", "plus", "search"].includes(name);
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill={stroke ? "none" : "currentColor"} stroke={stroke ? "currentColor" : "none"} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d={p[name]} fillRule="evenodd" />
    </svg>
  );
};
