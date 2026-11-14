---
name: ui-ux-pro-max
description: >-
  Senior desktop UI/UX guidelines for frameless utilities, hardware visualizers,
  accessible interactive color pickers, and low-noise control surfaces.
---

# UI/UX Pro Max — Hardware Utility Guidelines

1. **Hero-First Layout:**
   - The DualSense controller render and live reactive lightbar occupy the visual center of gravity.
   - All controls sit neatly below the hero in compact, logically grouped translucent capsules without scrollbars or visual clutter.

2. **Minimalist Draggable Chrome:**
   - Top bar (`data-tauri-drag-region`) contains brand mark, app name, live connection status pill (`USB`, `Bluetooth`, `Desconectado`), minimize-to-tray button, and close-to-tray button (`aria-label` on all icon buttons).
   - Interactive controls inside the titlebar must explicitly stop drag propagation (`data-tauri-drag-region="false"` or CSS `-webkit-app-region: no-drag`).

3. **Hardware-Friendly Color Interaction:**
   - Dragging a color wheel or brightness slider must throttle IPC commands (~30-60 Hz / `20ms`) to prevent saturating Bluetooth HID write queues while updating local React preview at 60+ fps.
   - Provide quick one-click preset capsules and an instant "Apagar LED" action.

4. **Accessibility:**
   - Visible keyboard focus rings (`focus-visible:ring-2 focus-visible:ring-sky-400`).
   - Support `prefers-reduced-motion` for ambient halo animations.
