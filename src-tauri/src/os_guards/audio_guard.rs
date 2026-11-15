//! USB Audio Hijack Guard: restaura la salida de audio previa cuando el DualSense (USB)
//! se convierte en el dispositivo predeterminado.

pub const DUALSENSE_AUDIO_NAME: &str = "DualSense Wireless Controller";

/// Gestor de audio del SO (mockeable).
pub trait AudioManager: Send {
    fn default_output(&self) -> Result<String, String>;
    fn set_default_output(&mut self, device: &str) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct AudioGuard {
    previous: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardAction {
    /// Se registró la salida actual como la preferida del usuario.
    Remembered(String),
    /// El mando secuestró el audio y se ha restaurado `restored_to`.
    Restored {
        restored_to: String,
    },
    Nothing,
}

fn is_dualsense(name: &str) -> bool {
    name.to_ascii_lowercase()
        .contains(&DUALSENSE_AUDIO_NAME.to_ascii_lowercase())
}

impl AudioGuard {
    /// Llamar periódicamente (y siempre al recibir `Connected(Usb)`).
    pub fn check<M: AudioManager>(&mut self, manager: &mut M) -> Result<GuardAction, String> {
        let current = manager.default_output()?;
        if !is_dualsense(&current) {
            let changed = self.previous.as_deref() != Some(current.as_str());
            self.previous = Some(current.clone());
            return Ok(if changed {
                GuardAction::Remembered(current)
            } else {
                GuardAction::Nothing
            });
        }
        match self.previous.clone() {
            Some(prev) => {
                manager.set_default_output(&prev)?;
                Ok(GuardAction::Restored { restored_to: prev })
            }
            None => Ok(GuardAction::Nothing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeAudio {
        current: String,
        sets: Vec<String>,
    }
    impl AudioManager for FakeAudio {
        fn default_output(&self) -> Result<String, String> {
            Ok(self.current.clone())
        }
        fn set_default_output(&mut self, device: &str) -> Result<(), String> {
            self.current = device.to_string();
            self.sets.push(device.to_string());
            Ok(())
        }
    }

    #[test]
    fn restores_previous_output_after_dualsense_usb_hijack() {
        let mut audio = FakeAudio {
            current: "Auriculares USB".into(),
            sets: vec![],
        };
        let mut guard = AudioGuard::default();
        assert_eq!(
            guard.check(&mut audio),
            Ok(GuardAction::Remembered("Auriculares USB".into()))
        );
        assert_eq!(guard.check(&mut audio), Ok(GuardAction::Nothing));
        audio.current = "Speakers (DualSense Wireless Controller)".into();
        assert_eq!(
            guard.check(&mut audio),
            Ok(GuardAction::Restored {
                restored_to: "Auriculares USB".into()
            })
        );
        assert_eq!(audio.current, "Auriculares USB");
    }

    #[test]
    fn does_nothing_when_no_previous_output_known() {
        let mut audio = FakeAudio {
            current: "DualSense Wireless Controller".into(),
            sets: vec![],
        };
        assert_eq!(
            AudioGuard::default().check(&mut audio),
            Ok(GuardAction::Nothing)
        );
        assert!(audio.sets.is_empty());
    }

    #[test]
    fn updates_remembered_device_when_user_switches_manually() {
        let mut audio = FakeAudio {
            current: "Altavoces HDMI".into(),
            sets: vec![],
        };
        let mut guard = AudioGuard::default();
        assert_eq!(
            guard.check(&mut audio),
            Ok(GuardAction::Remembered("Altavoces HDMI".into()))
        );
        audio.current = "Auriculares USB".into();
        assert_eq!(
            guard.check(&mut audio),
            Ok(GuardAction::Remembered("Auriculares USB".into()))
        );
        assert!(audio.sets.is_empty());
    }
}
