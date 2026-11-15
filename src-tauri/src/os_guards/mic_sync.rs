//! Sincronización botón Mute del DualSense ↔ micrófono del sistema (WASAPI / PipeWire).

use crate::hid::color::MicLedState;

/// Abstracción del gestor de audio del SO para poder testear sin hardware.
pub trait MicBackend: Send {
    fn set_system_mic_muted(&mut self, muted: bool) -> Result<(), String>;
    fn is_system_mic_muted(&self) -> Result<bool, String>;
}

/// Backend real de Linux/SteamOS vía `wpctl` (PipeWire).
#[derive(Debug, Default)]
pub struct PipeWireMic;

impl MicBackend for PipeWireMic {
    fn set_system_mic_muted(&mut self, muted: bool) -> Result<(), String> {
        let status = std::process::Command::new("wpctl")
            .args([
                "set-mute",
                "@DEFAULT_AUDIO_SOURCE@",
                if muted { "1" } else { "0" },
            ])
            .status()
            .map_err(|e| format!("wpctl no disponible: {e}"))?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| "wpctl devolvió error".into())
    }

    fn is_system_mic_muted(&self) -> Result<bool, String> {
        let out = std::process::Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
            .output()
            .map_err(|e| format!("wpctl no disponible: {e}"))?;
        Ok(String::from_utf8_lossy(&out.stdout).contains("[MUTED]"))
    }
}

/// Mantiene sincronizado el estado de mute entre mando y sistema.
#[derive(Debug, Default)]
pub struct MicSync {
    last_button: bool,
    muted: bool,
}

impl MicSync {
    /// Procesa el estado del botón físico (flanco de subida alterna el mute) y
    /// devuelve el LED naranja que debe mostrar el mando según el estado real del sistema.
    pub fn on_button_state<B: MicBackend>(
        &mut self,
        pressed: bool,
        backend: &mut B,
    ) -> Result<MicLedState, String> {
        if pressed && !self.last_button {
            let current = backend.is_system_mic_muted()?;
            backend.set_system_mic_muted(!current)?;
            self.muted = !current;
        }
        self.last_button = pressed;
        Ok(self.led())
    }

    /// Re-sincroniza el LED con el sistema (p. ej. si Discord silenció por su cuenta).
    pub fn refresh<B: MicBackend>(&mut self, backend: &B) -> Result<MicLedState, String> {
        self.muted = backend.is_system_mic_muted()?;
        Ok(self.led())
    }

    #[must_use]
    pub const fn led(&self) -> MicLedState {
        if self.muted {
            MicLedState::On
        } else {
            MicLedState::Off
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeMic {
        muted: bool,
        fail: bool,
    }
    impl MicBackend for FakeMic {
        fn set_system_mic_muted(&mut self, muted: bool) -> Result<(), String> {
            if self.fail {
                return Err("boom".into());
            }
            self.muted = muted;
            Ok(())
        }
        fn is_system_mic_muted(&self) -> Result<bool, String> {
            Ok(self.muted)
        }
    }

    #[test]
    fn button_press_toggles_system_mic_and_led() {
        let mut sync = MicSync::default();
        let mut mic = FakeMic::default();
        assert_eq!(sync.on_button_state(true, &mut mic), Ok(MicLedState::On));
        assert!(mic.muted);
        // Mantener pulsado no vuelve a alternar.
        assert_eq!(sync.on_button_state(true, &mut mic), Ok(MicLedState::On));
        sync.on_button_state(false, &mut mic).unwrap();
        assert_eq!(sync.on_button_state(true, &mut mic), Ok(MicLedState::Off));
        assert!(!mic.muted);
    }

    #[test]
    fn refresh_follows_external_mute_changes() {
        let mut sync = MicSync::default();
        let mic = FakeMic {
            muted: true,
            fail: false,
        };
        assert_eq!(sync.refresh(&mic), Ok(MicLedState::On));
    }

    #[test]
    fn backend_errors_are_propagated() {
        let mut sync = MicSync::default();
        let mut mic = FakeMic {
            muted: false,
            fail: true,
        };
        assert!(sync.on_button_state(true, &mut mic).is_err());
    }
}
