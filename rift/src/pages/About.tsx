import { Card, Logo } from "../components/ui";

export default function About() {
  return (
    <>
      <div className="hero"><Logo size={84} /><div><h1>Rift</h1><p className="tag">Optimize. Launch. Enjoy.</p><p className="muted">Version 0.1.0</p></div></div>
      <Card title="What Rift does">
        <ul className="tips">
          <li>Finds games from Steam, Epic, Minecraft, Roblox, VALORANT and folders you choose.</li>
          <li>Launches them through their official launchers (Steam, Epic, Riot, Minecraft Launcher).</li>
          <li>Applies a small set of real, reversible optimizations and keeps a backup of everything it changes.</li>
          <li>Manages Fabric profiles and performance mods for Minecraft Java through Modrinth.</li>
        </ul>
      </Card>
      <Card title="What Rift will never do">
        <ul className="tips">
          <li>Ask for an account, a login or a subscription.</li>
          <li>Touch anti-cheat, security services, or game files used for online play.</li>
          <li>Handle your Minecraft/Microsoft sign-in - that always stays in the official launcher.</li>
        </ul>
      </Card>
      <Card title="Credits"><p className="muted">Mods and metadata: Modrinth and the Fabric project. Rift is not affiliated with Mojang, Microsoft, Epic Games, Valve, Riot Games, Rockstar Games or Roblox. Game names are used only to identify installed software.</p></Card>
    </>
  );
}
