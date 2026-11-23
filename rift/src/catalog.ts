// Static knowledge about supported games. Whether a game is INSTALLED always comes from the real scanner.
export interface CatalogGame {
  key: string;
  name: string;
  hue: string; // tile gradient
  letter: string;
  launch: string; // how Rift launches it
  tips: string[]; // recommendations (advice only, nothing is changed by reading these)
  tools: ("minecraft" | "fortnite-settings" | "launch-args" | "power")[];
  mods: string;
}

export const CATALOG: CatalogGame[] = [
  {
    key: "minecraft-java", name: "Minecraft Java Edition", hue: "#3b8b2a,#6b4a2a", letter: "M",
    launch: "Official Minecraft Launcher (your Microsoft account)",
    tips: ["Install Sodium, Lithium and FerriteCore first - they are the biggest safe wins.", "4-6 GB of RAM is enough for most play; more can cause longer garbage-collection pauses.", "Lower render distance to 10-12 chunks on older hardware."],
    tools: ["minecraft", "power"], mods: "Fabric mods via Modrinth",
  },
  {
    key: "minecraft-bedrock", name: "Minecraft Bedrock Edition", hue: "#2a6f8b,#2a3b6b", letter: "B",
    launch: "Microsoft Store app",
    tips: ["Lower render distance and turn off fancy graphics for a quick FPS gain.", "Bedrock does not use Fabric; mods there are add-ons managed inside the game."],
    tools: ["power"], mods: "Not supported (Bedrock add-ons are managed in-game)",
  },
  {
    key: "fortnite", name: "Fortnite", hue: "#1e90ff,#6a2cff", letter: "F",
    launch: "Epic Games Launcher",
    tips: ["Use Performance mode (DX11/DX12 Performance) for the highest FPS on mid-range PCs.", "Set 3D resolution to 100% on native-res monitors; lower only if you are GPU-bound.", "Cap FPS slightly under your monitor refresh rate with VSync off.", "Close browsers and overlay apps before playing.", "Rift never touches anti-cheat or game files - only your own settings file, with a backup."],
    tools: ["fortnite-settings", "power"], mods: "Not supported",
  },
  {
    key: "roblox", name: "Roblox", hue: "#555a66,#23262e", letter: "R",
    launch: "roblox:// protocol (Roblox app)",
    tips: ["Lower Graphics Quality in the in-game menu (Esc > Settings).", "Roblox controls its own FPS cap; Rift does not modify Roblox files.", "Close heavy browser tabs - Roblox is CPU/RAM sensitive."],
    tools: ["power"], mods: "Not supported",
  },
  {
    key: "gta5", name: "GTA V", hue: "#2f7d4a,#16261c", letter: "V",
    launch: "Steam / Epic / Rockstar Launcher",
    tips: ["Turn Grass and Shadows down first - they cost the most FPS.", "Use DX11 unless you need the DX10 fallback.", "Keep Population Density moderate on 6 GB or less VRAM."],
    tools: ["launch-args", "power"], mods: "Single-player mods only; Rift does not install them (online anti-cheat risk).",
  },
  {
    key: "gta4", name: "GTA IV", hue: "#8a6b2a,#2a2216", letter: "IV",
    launch: "Steam / Rockstar Launcher",
    tips: ["Keep Detail Distance and View Distance sliders conservative - the engine scales poorly.", "Use the Games for Windows / Steam compatibility notes for your version."],
    tools: ["launch-args", "power"], mods: "Not supported",
  },
  {
    key: "valorant", name: "VALORANT", hue: "#ff4655,#2a0f14", letter: "V",
    launch: "Riot Client",
    tips: ["Set Material, Texture and Detail quality to Low and turn off Vignette/Bloom for FPS.", "Use Fullscreen + your monitor's native resolution; Rift does not change Valorant files.", "Vanguard must run - Rift never disables it."],
    tools: ["power"], mods: "Not supported (anti-cheat)",
  },
  {
    key: "cs2", name: "Counter-Strike 2", hue: "#e8a33d,#2a2012", letter: "CS",
    launch: "Steam (with Rift launch options)",
    tips: ["-novid skips the intro video; -high asks Windows for high CPU priority.", "Use Fullscreen exclusive and limit FPS above your refresh rate.", "Lower Shadow and Model/Texture detail first on weaker GPUs."],
    tools: ["launch-args", "power"], mods: "Not supported (VAC)",
  },
  {
    key: "terraria", name: "Terraria", hue: "#3aa655,#1b2f6b", letter: "T",
    launch: "Steam",
    tips: ["Turn off Lighting quality 'Retro/Trippy' only if you see stutter.", "tModLoader is a separate Steam app - install it from Steam to use mods."],
    tools: ["launch-args", "power"], mods: "Via tModLoader (separate app)",
  },
];

export const GENERIC: CatalogGame = {
  key: "generic", name: "Game", hue: "#873bff,#2a1a4a", letter: "G", launch: "Direct executable",
  tips: ["Run the game in fullscreen at your monitor's native resolution.", "Close browsers and overlays before playing.", "Keep GPU drivers up to date."],
  tools: ["launch-args", "power"], mods: "Not available for this game",
};
export const catalogFor = (key: string) => CATALOG.find((c) => c.key === key);
export const fmtBytes = (n: number) => (n > 1048576 ? (n / 1048576).toFixed(1) + " MB" : (n / 1024).toFixed(0) + " KB");
export const fmtTime = (ts: number) => (ts ? new Date(ts * 1000).toLocaleString() : "Never");
