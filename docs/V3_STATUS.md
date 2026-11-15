# SenseHue v3.0 — Estado de implementación

Principio **Zero Memory Injection**: solo APIs locales oficiales (GSI/HTTP, UDP, TCP OpenRGB), eventos de SO y detección de procesos.

| Pilar / Issue | Módulo | Estado |
|---|---|---|
| SimRacing RPM (F1/Forza) | `src-tauri/src/integrations/sim_telemetry.rs` | Parsers + mapa RPM→5 LEDs + banderas, testeado. Pendiente: bucle `UdpSocket` conectado al `EffectEngine`. |
| Valve GSI + LoL Live | `valve_gsi.rs`, `lol_live.rs` | Parsers + lógica de frames (flash con límite fotosensible), testeado. Pendiente: listener HTTP/polling conectado a la app. |
| OpenRGB + WLED | `openrgb_client.rs`, `wled_udp.rs` | Protocolo `ORGB` y `DRGB` con mock server TCP/UDP, testeado. Pendiente: toggle en UI. |
| Gatillos competitivos + Gyro | `hid/triggers.rs` | `HairTrigger`, `DualStage`, `ProgressivePedal` + `gyro_indicator_color`, testeado. |
| Mic Mute Sync + Audio Guard | `os_guards/` | Lógica trait-based testeada; backend real solo PipeWire (`wpctl`). Pendiente: backend WASAPI en Windows. |
| SteamOS | `src/systemd/sensehue.service`, `integrations/decky-plugin/` | Servicio y plugin Decky (sin compilar con Decky CLI). |
| Discover (comunidad) | `community/registry.rs` | Validación estricta de índice y perfiles, testeado. Pendiente: pestaña UI + descarga HTTPS y el repo `sensehue-presets`. |
| Autostart | `tauri-plugin-autostart` | Integrado. |
| Auto-updater firmado | — | **Bloqueado**: requiere generar y custodiar la clave de firma (`tauri signer generate`) y un endpoint de releases. |
| Winget / Flathub / AUR | `packaging/` | Manifiestos listos; los `sha256` se rellenan al publicar la release v3.0.0. |
