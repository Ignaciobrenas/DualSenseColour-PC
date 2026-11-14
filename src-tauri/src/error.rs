use serde::Serialize;
use thiserror::Error;

/// Typed errors across HID communication, configuration persistence, and Tauri IPC.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("No se detectó ningún mando DualSense conectado")]
    ControllerNotFound,

    #[error("Permisos insuficientes para acceder al dispositivo HID: {0}")]
    PermissionDenied(String),

    #[error("Error de comunicación HID: {0}")]
    HidIo(String),

    #[error("Color inválido: {0}")]
    InvalidColor(String),

    #[error("Error de configuración: {0}")]
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
