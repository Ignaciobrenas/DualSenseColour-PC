//! Local HTTP API server for external integrations (Stream Deck, OBS Studio, AutoHotkey).
//!
//! Strictly bound to loopback (`127.0.0.1:17420`) and disabled by default in `AppConfig`.
//! Zero external HTTP framework overhead: powered directly by `tokio::net::TcpListener`.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::effects::LightingEffectMode;
use crate::hid::color::RgbColor;

/// Default loopback socket address for the optional SenseHue Local API.
pub const DEFAULT_LOCAL_API_ADDR: &str = "127.0.0.1:17420";

/// Parsed command emitted by an incoming Local API HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalApiCommand {
    /// `GET /status`
    GetStatus,
    /// `POST /preset/<id>`
    SetPreset(String),
    /// `POST /color/<rrggbb>`
    SetColor(RgbColor),
    /// `POST /mode/<mode>`
    SetMode(LightingEffectMode),
    /// `POST /off`
    TurnOff,
}

/// HTTP response returned by the Local API router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalApiResponse {
    pub status_code: u16,
    pub body: String,
}

impl LocalApiResponse {
    /// Builds a `200 OK` JSON response.
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status_code: 200,
            body: body.into(),
        }
    }

    /// Builds an error JSON response with the given HTTP status code.
    pub fn error(status_code: u16, message: impl Into<String>) -> Self {
        let escaped = message.into().replace('"', "\\\"");
        Self {
            status_code,
            body: format!("{{\"ok\":false,\"error\":\"{escaped}\"}}"),
        }
    }

    /// Serializes this response into a minimal HTTP/1.1 wire payload.
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let reason = match self.status_code {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            _ => "Internal Server Error",
        };
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status_code,
            reason,
            self.body.len(),
            self.body
        )
        .into_bytes()
    }
}

/// Parses a 6-digit hex color string (with or without leading `#`) into [`RgbColor`].
pub fn parse_hex_color(hex: &str) -> Option<RgbColor> {
    let trimmed = hex.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some(RgbColor::new(r, g, b))
}

/// Resolves built-in SenseHue preset identifiers for `POST /preset/<id>`.
pub fn preset_color_for_id(id: &str) -> Option<RgbColor> {
    match id.trim().to_ascii_lowercase().as_str() {
        "ps5-blue" | "blue" => Some(RgbColor::new(0, 67, 156)),
        "ice" | "cyan" | "sensehue" => Some(RgbColor::new(0, 229, 255)),
        "crimson" | "red" => Some(RgbColor::new(220, 38, 38)),
        "cosmic-purple" | "purple" => Some(RgbColor::new(168, 85, 247)),
        "emerald" | "green" => Some(RgbColor::new(16, 185, 129)),
        "amber" | "gold" => Some(RgbColor::new(245, 158, 11)),
        _ => None,
    }
}

/// Parses an HTTP/1.1 request line into either a validated [`LocalApiCommand`] or error response.
pub fn parse_http_request(raw_request: &str) -> Result<LocalApiCommand, LocalApiResponse> {
    let request_line = raw_request
        .lines()
        .next()
        .ok_or_else(|| LocalApiResponse::error(400, "Empty HTTP request"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| LocalApiResponse::error(400, "Missing HTTP method"))?;
    let path = parts
        .next()
        .ok_or_else(|| LocalApiResponse::error(400, "Missing request path"))?;
    let clean_path = path.split('?').next().unwrap_or(path).trim_end_matches('/');

    match (method, clean_path) {
        ("GET", "/status") => Ok(LocalApiCommand::GetStatus),
        ("POST", "/off") => Ok(LocalApiCommand::TurnOff),
        ("POST", p) if p.starts_with("/preset/") => {
            let id = p.trim_start_matches("/preset/");
            if id.is_empty() || preset_color_for_id(id).is_none() {
                return Err(LocalApiResponse::error(
                    404,
                    format!("Unknown preset: {id}"),
                ));
            }
            Ok(LocalApiCommand::SetPreset(id.to_ascii_lowercase()))
        }
        ("POST", p) if p.starts_with("/color/") => {
            let hex = p.trim_start_matches("/color/");
            let rgb = parse_hex_color(hex)
                .ok_or_else(|| LocalApiResponse::error(400, format!("Invalid hex color: {hex}")))?;
            Ok(LocalApiCommand::SetColor(rgb))
        }
        ("POST", p) if p.starts_with("/mode/") => {
            let slug = p.trim_start_matches("/mode/");
            let mode = LightingEffectMode::parse(slug)
                .ok_or_else(|| LocalApiResponse::error(400, format!("Unknown mode: {slug}")))?;
            Ok(LocalApiCommand::SetMode(mode))
        }
        ("GET", _) | ("POST", _) => Err(LocalApiResponse::error(404, "Endpoint not found")),
        _ => Err(LocalApiResponse::error(405, "Method not allowed")),
    }
}

/// Spawns the non-blocking loopback HTTP listener when `local_api_enabled` is turned on.
pub fn spawn_local_api_server<F>(addr: &str, handler: F) -> tauri::async_runtime::JoinHandle<()>
where
    F: Fn(LocalApiCommand) -> LocalApiResponse + Send + Sync + 'static,
{
    let bind_addr = addr.to_string();
    let handler = Arc::new(handler);

    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind(&bind_addr).await {
            Ok(l) => l,
            Err(err) => {
                tracing::warn!("SenseHue Local API failed to bind on {bind_addr}: {err}");
                return;
            }
        };
        tracing::info!("SenseHue Local API listening on http://{bind_addr}");

        loop {
            let Ok((mut stream, _peer)) = listener.accept().await else {
                continue;
            };
            let handler = Arc::clone(&handler);
            tauri::async_runtime::spawn(async move {
                let mut buf = [0u8; 2048];
                let Ok(n) = stream.read(&mut buf).await else {
                    return;
                };
                if n == 0 {
                    return;
                }
                let raw = String::from_utf8_lossy(&buf[..n]);
                let response = match parse_http_request(&raw) {
                    Ok(cmd) => handler(cmd),
                    Err(err_resp) => err_resp,
                };
                let _ = stream.write_all(&response.to_http_bytes()).await;
            });
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_and_off_routes() {
        assert_eq!(
            parse_http_request("GET /status HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            Ok(LocalApiCommand::GetStatus)
        );
        assert_eq!(
            parse_http_request("POST /off HTTP/1.1\r\n\r\n"),
            Ok(LocalApiCommand::TurnOff)
        );
    }

    #[test]
    fn parses_color_preset_and_mode_routes() {
        assert_eq!(
            parse_http_request("POST /color/00E5FF HTTP/1.1\r\n\r\n"),
            Ok(LocalApiCommand::SetColor(RgbColor::new(0, 229, 255)))
        );
        assert_eq!(
            parse_http_request("POST /preset/crimson HTTP/1.1\r\n\r\n"),
            Ok(LocalApiCommand::SetPreset("crimson".to_string()))
        );
        assert_eq!(
            parse_http_request("POST /mode/spectrum HTTP/1.1\r\n\r\n"),
            Ok(LocalApiCommand::SetMode(LightingEffectMode::Spectrum))
        );
    }

    #[test]
    fn rejects_invalid_hex_and_unknown_routes() {
        let bad_hex = parse_http_request("POST /color/ZZZZZZ HTTP/1.1\r\n\r\n").unwrap_err();
        assert_eq!(bad_hex.status_code, 400);

        let missing = parse_http_request("GET /unknown HTTP/1.1\r\n\r\n").unwrap_err();
        assert_eq!(missing.status_code, 404);
    }
}
