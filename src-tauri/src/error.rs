use serde::Serialize;
use thiserror::Error;

/// Typed errors across HID communication, configuration persistence, and Tauri IPC.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("No se detectó ningún mando DualSense. Conéctalo por USB o Bluetooth, enciéndelo y pulsa Reintentar.")]
    ControllerNotFound,

    #[error("Sin permisos para acceder al mando ({0}). En Linux revisa las reglas udev; en Windows cierra apps exclusivas.")]
    PermissionDenied(String),

    #[error("No se pudo comunicar con el mando: {0}")]
    HidIo(String),

    #[error("El formato del color no es válido ({0}). Usa un código hexadecimal como #00E5FF.")]
    InvalidColor(String),

    #[error("No se pudo leer o guardar la configuración: {0}")]
    Config(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
