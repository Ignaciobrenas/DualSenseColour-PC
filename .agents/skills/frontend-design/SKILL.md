---
name: frontend-design
description: >-
  High-craft desktop and web UI design system focused on glassmorphism, layered
  depth, WCAG contrast on translucent surfaces, and tactile micro-interactions.
---

# Frontend Design Principles (SenseHue)

1. **Spatial Hierarchy & Framing:**
   - Compact frameless desktop canvas (`~440x680px`), rounded outer shell (`border-radius: 22px`), crisp 1px specular rim (`rgba(255, 255, 255, 0.14)`).
   - 60% deep obsidian glass background (`rgba(10, 12, 18, 0.78)`), 30% elevated frosted modules (`rgba(255, 255, 255, 0.045)`), 10% dynamic LED color accents driven directly by the active DualSense color.

2. **Glassmorphism Real Spec:**
   - `backdrop-filter: blur(22px) saturate(140%)`.
   - Specular top highlight (`inset 0 1px 0 rgba(255, 255, 255, 0.14)`) and soft ambient glow matching the live LED color.
   - Maintain WCAG AA text contrast (`>= 4.5:1`) using `#F8FAFC` for primary labels and `#94A3B8` / `#CBD5E1` for secondary metadata.

3. **Typography & Precision:**
   - UI Sans (`Inter`, `Plus Jakarta Sans`, system-ui) for controls.
   - Tabular monospace (`JetBrains Mono`, `Cascadia Code`, monospace) for HEX codes, RGB triplets, and brightness percentages so numbers never jitter when scrubbing.

4. **Tactile Transitions:**
   - Standard UI transitions: `200ms cubic-bezier(0.22, 1, 0.36, 1)`.
   - LED color transitions must feel responsive without visual stutter (`transition: filter 120ms ease-out, background-color 120ms ease-out`).
