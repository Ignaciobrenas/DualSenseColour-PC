# SenseHue v3.0 — Estado de implementación

Principio **Zero Memory Injection**: solo APIs locales oficiales (GSI/HTTP, UDP, TCP OpenRGB), eventos de SO y detección de procesos.

| Pilar / Issue | Módulo | Estado |
|---|---|---|
| SimRacing RPM (F1/Forza) (#41) | `sim_telemetry.rs`, `integrations/runtime.rs` | Completado y testeado (bucle UDP conectado al motor de efectos). |
| Valve GSI + LoL Live (#42) | `valve_gsi.rs`, `lol_live.rs`, `integrations/runtime.rs` | Completado y testeado (endpoint HTTP loopback y flash fotosensible limitado). |
| OpenRGB + WLED (#43) | `openrgb_client.rs`, `wled_udp.rs` | Completado y testeado (cliente TCP `ORGB`, UDP `DRGB` y toggles en UI). |
| Gatillos competitivos + Gyro (#44) | `hid/triggers.rs` | Completado y testeado (`HairTrigger`, `DualStage`, `ProgressivePedal` + indicador Gyro). |
| Mic Mute Sync + Audio Guard (#45) | `os_guards/` | Completado y testeado con traits (PipeWire/WASAPI y guardia anti-secuestro USB). |
| SteamOS + Decky (#46) | `src/systemd/sensehue.service`, `integrations/decky-plugin/` | Servicio y plugin Decky listos (marcas limpias y ESLint sin avisos). |
| Discover (comunidad) (#47) | `community/registry.rs`, `components/DiscoverTab.tsx` | Completado (esquema estricto sin campos ejecutables y pestaña UI). |
| Autostart + Updater (#48) | `tauri-plugin-autostart`, `tauri-plugin-single-instance` | Autostart e instancia única activos; updater firmado pendiente de clave Ed25519 en CI. |
| Winget / Flathub / AUR (#49) | `packaging/`, `src-tauri/nsis/hooks.nsh` | Manifiestos e instalador NSIS personalizado con marca y bordes redondeados listos; hashes SHA-256 tras publicar release. |
| QA Zero-Injection & v3.0 (#50) | App completa (`monitor.rs`, `tray.rs`, `TERMS_AND_CONDITIONS.txt`) | Auditoría Zero-Injection, soporte multi-mando USB/Bluetooth, batería real + alertas tray (20/10/5 %) e ilustración SVG propia verificados. |
