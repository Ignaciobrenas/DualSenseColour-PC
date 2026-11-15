//! Cliente binario del SDK de OpenRGB (TCP `6742`, cabecera `ORGB`).

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::hid::color::RgbColor;

pub const OPENRGB_PORT: u16 = 6742;
pub const MAGIC: &[u8; 4] = b"ORGB";
pub const CLIENT_PROTOCOL_VERSION: u32 = 3;

pub const CMD_REQUEST_CONTROLLER_COUNT: u32 = 0;
pub const CMD_REQUEST_PROTOCOL_VERSION: u32 = 40;
pub const CMD_SET_CLIENT_NAME: u32 = 50;
pub const CMD_RGBCONTROLLER_UPDATELEDS: u32 = 1050;

/// Construye un paquete OpenRGB: `ORGB | dev_idx u32 | pkt_id u32 | size u32 | payload` (LE).
#[must_use]
pub fn build_packet(device_index: u32, packet_id: u32, payload: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(16 + payload.len());
    p.extend_from_slice(MAGIC);
    p.extend_from_slice(&device_index.to_le_bytes());
    p.extend_from_slice(&packet_id.to_le_bytes());
    p.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    p.extend_from_slice(payload);
    p
}

/// Paquete `SET_CLIENT_NAME` (cadena terminada en NUL).
#[must_use]
pub fn build_set_client_name(name: &str) -> Vec<u8> {
    let mut payload = name.as_bytes().to_vec();
    payload.push(0);
    build_packet(0, CMD_SET_CLIENT_NAME, &payload)
}

/// Paquete `RGBCONTROLLER_UPDATELEDS`: `data_size u32 | num_colors u16 | (r g b 0)*n`.
#[must_use]
pub fn build_update_leds(device_index: u32, color: RgbColor, led_count: u16) -> Vec<u8> {
    let data_size = 4 + 2 + u32::from(led_count) * 4;
    let mut payload = Vec::with_capacity(data_size as usize);
    payload.extend_from_slice(&data_size.to_le_bytes());
    payload.extend_from_slice(&led_count.to_le_bytes());
    for _ in 0..led_count {
        payload.extend_from_slice(&[color.r, color.g, color.b, 0]);
    }
    build_packet(device_index, CMD_RGBCONTROLLER_UPDATELEDS, &payload)
}

/// Cabecera decodificada de un paquete recibido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub device_index: u32,
    pub packet_id: u32,
    pub size: u32,
}

/// Decodifica una cabecera de 16 bytes validando la firma `ORGB`.
#[must_use]
pub fn parse_header(buf: &[u8]) -> Option<Header> {
    if buf.len() < 16 || &buf[..4] != MAGIC {
        return None;
    }
    let u = |o: usize| u32::from_le_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]);
    Some(Header {
        device_index: u(4),
        packet_id: u(8),
        size: u(12),
    })
}

/// Modo **Sync-Out**: replica `color` en `device_count` dispositivos del servidor OpenRGB.
pub async fn sync_out(
    addr: &str,
    color: RgbColor,
    device_count: u32,
    led_count: u16,
) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(addr).await?;
    stream.write_all(&build_set_client_name("SenseHue")).await?;
    for dev in 0..device_count {
        stream
            .write_all(&build_update_leds(dev, color, led_count))
            .await?;
    }
    stream.flush().await
}

/// Pregunta al servidor cuántos controladores hay (base del modo **Sync-In**).
pub async fn request_controller_count(addr: &str) -> std::io::Result<u32> {
    let mut stream = TcpStream::connect(addr).await?;
    stream
        .write_all(&build_packet(0, CMD_REQUEST_CONTROLLER_COUNT, &[]))
        .await?;
    let mut header = [0u8; 16];
    stream.read_exact(&mut header).await?;
    let h = parse_header(&header).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "respuesta sin firma ORGB")
    })?;
    if h.size < 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "payload demasiado corto",
        ));
    }
    let mut payload = vec![0u8; h.size as usize];
    stream.read_exact(&mut payload).await?;
    Ok(u32::from_le_bytes([
        payload[0], payload[1], payload[2], payload[3],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn packet_header_layout_is_orgb_le() {
        let p = build_packet(2, 1050, &[1, 2, 3]);
        assert_eq!(&p[..4], b"ORGB");
        assert_eq!(
            parse_header(&p),
            Some(Header {
                device_index: 2,
                packet_id: 1050,
                size: 3
            })
        );
        assert_eq!(&p[16..], &[1, 2, 3]);
    }

    #[test]
    fn update_leds_payload_is_correct() {
        let p = build_update_leds(1, RgbColor::new(10, 20, 30), 2);
        let h = parse_header(&p).unwrap();
        assert_eq!(h.packet_id, CMD_RGBCONTROLLER_UPDATELEDS);
        assert_eq!(h.size, 14);
        assert_eq!(&p[16..20], &14u32.to_le_bytes());
        assert_eq!(&p[20..22], &2u16.to_le_bytes());
        assert_eq!(&p[22..], &[10, 20, 30, 0, 10, 20, 30, 0]);
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(parse_header(&[0u8; 16]).is_none());
        assert!(parse_header(b"ORGB").is_none());
    }

    #[tokio::test]
    async fn sync_out_talks_to_mock_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut all = Vec::new();
            s.read_to_end(&mut all).await.unwrap();
            all
        });
        sync_out(&addr, RgbColor::new(1, 2, 3), 1, 1).await.unwrap();
        let data = server.await.unwrap();
        let name = build_set_client_name("SenseHue");
        assert_eq!(&data[..name.len()], name.as_slice());
        let h = parse_header(&data[name.len()..]).unwrap();
        assert_eq!(h.packet_id, CMD_RGBCONTROLLER_UPDATELEDS);
    }

    #[tokio::test]
    async fn reads_controller_count_from_mock_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut req = [0u8; 16];
            s.read_exact(&mut req).await.unwrap();
            s.write_all(&build_packet(
                0,
                CMD_REQUEST_CONTROLLER_COUNT,
                &5u32.to_le_bytes(),
            ))
            .await
            .unwrap();
        });
        assert_eq!(request_controller_count(&addr).await.unwrap(), 5);
    }
}
