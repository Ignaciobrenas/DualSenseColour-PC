import { definePlugin } from "@decky/api";
import { DeckyContent } from "./Content";

const SenseHueDeckyPlugin = definePlugin(() => ({
  name: "SenseHue",
  titleView: <div>SenseHue</div>,
  content: <DeckyContent />,
  icon: <span>SH</span>,
  onDismount() {},
}));

export default SenseHueDeckyPlugin;
