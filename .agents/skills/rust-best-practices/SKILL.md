---
name: rust-best-practices
description: >-
  Idiomatic Rust architecture for hardware HID communication, trait-based testing,
  error handling with thiserror, and deadlock-free async monitoring with tokio.
---

# Rust Best Practices (DualSenseColour PC)

1. **Trait Abstraction for Hardware (`HidTransport`):**
   - Isolate `hidapi` behind a `Send + Sync` trait (`HidTransport`) so protocol encoding, hotplug state transitions, and error recovery are 100% testable in CI without physical DualSense hardware.

2. **Typed Errors (`thiserror`) & Structured Logging (`tracing`):**
   - Never `.unwrap()` or `.expect()` in production paths (only inside `#[cfg(test)]`).
   - Surface actionable errors (`ControllerNotFound`, `PermissionDenied`, `IoError`, `InvalidColor`, `ConfigError`) cleanly across Tauri IPC.

3. **Concurrency & Locking Discipline:**
   - Never hold a synchronous `std::sync::Mutex` across an `.await` point.
   - Clone small immutable snapshots (`RgbColor`, `ControllerStatus`, `AppConfig`) before sending HID packets or emitting Tauri events.

4. **Protocol Determinism:**
   - All packet builders (`build_usb_report`, `build_bt_report`) must be pure functions over explicit inputs and verified byte-for-byte with unit tests including known CRC-32 test vectors (`0xA2` seed).
