//! Emisor WLED Realtime UDP (protocolo `DRGB`, puerto 21324).

use tokio::net::UdpSocket;

use crate::hid::color::RgbColor;

pub const WLED_UDP_PORT: u16 = 21324;
/// Protocolo DRGB: hasta 490 LEDs por datagrama.
pub const DRGB_PROTOCOL: u8 = 2;
pub const DRGB_MAX_LEDS: usize = 490;

/// Construye un paquete `DRGB`: `[2, timeout_s, r, g, b, ...]`.
///
/// `leds` se trunca a [`DRGB_MAX_LEDS`]. `timeout_s = 255` mantiene el modo realtime indefinidamente.
#[must_use]
pub fn build_drgb_packet(color: RgbColor, led_count: usize, timeout_s: u8) -> Vec<u8> {
    let n = led_count.clamp(1, DRGB_MAX_LEDS);
    let mut p = Vec::with_capacity(2 + n * 3);
    p.push(DRGB_PROTOCOL);
    p.push(timeout_s);
    for _ in 0..n {
        p.extend_from_slice(&[color.r, color.g, color.b]);
    }
    p
}

/// Envía el color a una tira WLED (`host:21324`).
pub async fn send_color(target: &str, color: RgbColor, led_count: usize) -> std::io::Result<usize> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket
        .send_to(&build_drgb_packet(color, led_count, 2), target)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drgb_header_and_payload_layout() {
        let p = build_drgb_packet(RgbColor::new(1, 2, 3), 3, 255);
        assert_eq!(p, vec![2, 255, 1, 2, 3, 1, 2, 3, 1, 2, 3]);
    }

    #[test]
    fn drgb_clamps_led_count() {
        assert_eq!(build_drgb_packet(RgbColor::new(0, 0, 0), 0, 1).len(), 5);
        assert_eq!(
            build_drgb_packet(RgbColor::new(0, 0, 0), 10_000, 1).len(),
            2 + DRGB_MAX_LEDS * 3
        );
    }

    #[tokio::test]
    async fn sends_drgb_datagram_to_mock_wled() {
        let mock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = mock.local_addr().unwrap().to_string();
        send_color(&addr, RgbColor::new(9, 8, 7), 2).await.unwrap();
        let mut buf = [0u8; 64];
        let (n, _) = mock.recv_from(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], &[2, 2, 9, 8, 7, 9, 8, 7]);
    }

    #[test]
    fn drgb_preserves_wait_timeout_byte_per_packet() {
        let packet = build_drgb_packet(RgbColor::new(12, 34, 56), 1, 7);
        assert_eq!(packet, vec![2, 7, 12, 34, 56]);
    }
}
