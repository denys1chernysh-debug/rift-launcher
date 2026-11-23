import { createContext, useContext } from "react";
import type { Store } from "./api";

export type Page = "home" | "games" | "optimizer" | "mods" | "downloads" | "settings" | "about" | "game";

export interface Ctx {
  store: Store;
  refresh: () => Promise<void>;
  go: (p: Page, gameKey?: string) => void;
  openGame: (key: string) => void;
  toast: (msg: string, kind?: "ok" | "err") => void;
  notify: (msg: string) => void;
}

export const AppCtx = createContext<Ctx>(null as unknown as Ctx);
export const useApp = () => useContext(AppCtx);
