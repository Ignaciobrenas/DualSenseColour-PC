import { definePlugin, callable } from "@decky/api";
import { PanelSection, PanelSectionRow, ButtonItem, ToggleField } from "@decky/ui";
import { useState } from "react";

const setPreset = callable<[string], unknown>("set_preset");
const nightMode = callable<[boolean], unknown>("night_mode");

const PRESETS: [string, string][] = [
  ["playstation-blue", "PlayStation Blue"],
  ["crimson-red", "Crimson Red"],
  ["cyber-cyan", "Cyber Cyan"],
];

function Content() {
  const [night, setNight] = useState(false);
  return (
    <PanelSection title="SenseHue">
      {PRESETS.map(([id, name]) => (
        <PanelSectionRow key={id}>
          <ButtonItem layout="below" onClick={() => setPreset(id)}>
            {name}
          </ButtonItem>
        </PanelSectionRow>
      ))}
      <PanelSectionRow>
        <ToggleField
          label="Modo Noche"
          checked={night}
          onChange={(v: boolean) => {
            setNight(v);
            nightMode(v);
          }}
        />
      </PanelSectionRow>
    </PanelSection>
  );
}

export default definePlugin(() => ({
  name: "SenseHue",
  titleView: <div>SenseHue</div>,
  content: <Content />,
  icon: <span>🎮</span>,
  onDismount() {},
}));
