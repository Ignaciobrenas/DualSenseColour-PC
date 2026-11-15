//! Runtime asíncrono de integraciones: listeners UDP (simracing) y HTTP local (Valve GSI).
//!
//! Solo escucha en loopback / puertos de telemetría pública. Sin inyección en procesos de juego.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};

use super::sim_telemetry::{
    parse_f1_car_status, parse_f1_car_telemetry, parse_forza_dash, rpm_frame, RpmSample,
    F1_HEADER_LEN, F1_PACKET_CAR_STATUS, F1_PACKET_CAR_TELEMETRY,
};
use super::valve_gsi::{gsi_frame, parse_gsi};
use super::LedFrame;

pub const GSI_ADDR: &str = "127.0.0.1:3000";
const MAX_GSI_BYTES: usize = 64 * 1024;

/// Destino de los frames producidos por las integraciones.
pub type FrameSink = Arc<dyn Fn(LedFrame) + Send + Sync>;

/// Escucha telemetría UDP de F1 en `bind` y emite un frame por cada paquete de telemetría.
pub async fn run_f1_listener(bind: &str, sink: FrameSink) -> std::io::Result<()> {
    let socket = UdpSocket::bind(bind).await?;
    tracing::info!("Telemetría F1 escuchando en udp://{bind}");
    serve_f1(socket, sink).await
}

async fn serve_f1(socket: UdpSocket, sink: FrameSink) -> std::io::Result<()> {
    let mut max_rpm = 13_000.0_f32;
    let mut pit = false;
    let mut buf = vec![0u8; 2048];
    loop {
        let (n, _) = socket.recv_from(&mut buf).await?;
        let pkt = &buf[..n];
        if n < F1_HEADER_LEN {
            continue;
        }
        match pkt[6] {
            F1_PACKET_CAR_STATUS => {
                if let Ok((m, p)) = parse_f1_car_status(pkt) {
                    if m > 0 {
                        max_rpm = f32::from(m);
                    }
                    pit = p;
                }
            }
            F1_PACKET_CAR_TELEMETRY => {
                if let Ok(rpm) = parse_f1_car_telemetry(pkt) {
                    sink(rpm_frame(&RpmSample {
                        rpm: f32::from(rpm),
                        max_rpm,
                        pit_limiter: pit,
                        yellow_flag: false,
                    }));
                }
            }
            _ => {}
        }
    }
}

/// Escucha el "Data Out" de Forza en `bind`.
pub async fn run_forza_listener(bind: &str, sink: FrameSink) -> std::io::Result<()> {
    let socket = UdpSocket::bind(bind).await?;
    tracing::info!("Telemetría Forza escuchando en udp://{bind}");
    let mut buf = vec![0u8; 2048];
    loop {
        let (n, _) = socket.recv_from(&mut buf).await?;
        if let Ok(sample) = parse_forza_dash(&buf[..n]) {
            sink(rpm_frame(&sample));
        }
    }
}

/// Servidor HTTP loopback para Valve GSI. Responde 200 a cada POST válido.
pub async fn run_gsi_server(bind: &str, sink: FrameSink) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    tracing::info!("Valve GSI escuchando en http://{bind}");
    let last_flash = Arc::new(std::sync::Mutex::new(None::<u64>));
    let started = std::time::Instant::now();
    loop {
        let (mut stream, _) = listener.accept().await?;
        let sink = Arc::clone(&sink);
        let last_flash = Arc::clone(&last_flash);
        tokio::spawn(async move {
            let Some(body) = read_http_body(&mut stream).await else {
                return;
            };
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await;
            if let Some(state) = parse_gsi(&body) {
                let now = started.elapsed().as_millis() as u64;
                let prev = last_flash.lock().ok().and_then(|g| *g);
                if let Some(frame) = gsi_frame(&state, now, prev) {
                    if frame.color == Some(crate::hid::color::RgbColor::new(255, 255, 255)) {
                        if let Ok(mut g) = last_flash.lock() {
                            *g = Some(now);
                        }
                    }
                    sink(frame);
                }
            }
        });
    }
}

async fn read_http_body(stream: &mut tokio::net::TcpStream) -> Option<String> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut chunk))
            .await
            .ok()?
            .ok()?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..n]);
        if data.len() > MAX_GSI_BYTES {
            return None;
        }
        if let Some(pos) = find_header_end(&data) {
            let headers = String::from_utf8_lossy(&data[..pos]).to_ascii_lowercase();
            let len = headers
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if data.len() >= pos + 4 + len {
                return Some(String::from_utf8_lossy(&data[pos + 4..pos + 4 + len]).into_owned());
            }
        }
    }
    None
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn collector() -> (FrameSink, Arc<Mutex<Vec<LedFrame>>>) {
        let store = Arc::new(Mutex::new(Vec::new()));
        let s = Arc::clone(&store);
        (Arc::new(move |f| s.lock().unwrap().push(f)), store)
    }

    async fn wait_for(store: &Arc<Mutex<Vec<LedFrame>>>) -> LedFrame {
        for _ in 0..100 {
            if let Some(f) = store.lock().unwrap().first().copied() {
                return f;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("no llegó ningún frame");
    }

    #[tokio::test]
    async fn f1_udp_end_to_end_produces_rpm_bar() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        let (sink, store) = collector();
        tokio::spawn(serve_f1(socket, sink));

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let mut status = vec![0u8; F1_HEADER_LEN + 22 * 55];
        status[6] = F1_PACKET_CAR_STATUS;
        status[F1_HEADER_LEN + 17..F1_HEADER_LEN + 19].copy_from_slice(&10_000u16.to_le_bytes());
        client.send_to(&status, addr).await.unwrap();
        let mut tel = vec![0u8; F1_HEADER_LEN + 22 * 60];
        tel[6] = F1_PACKET_CAR_TELEMETRY;
        let off = F1_HEADER_LEN + 2 + 12 + 2;
        tel[off..off + 2].copy_from_slice(&9_800u16.to_le_bytes());
        client.send_to(&tel, addr).await.unwrap();

        let frame = wait_for(&store).await;
        assert_eq!(frame.player_mask, Some(0b11111));
        assert!(frame.blink);
    }

    #[tokio::test]
    async fn gsi_http_end_to_end_flags_bomb() {
        let probe = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap().to_string();
        drop(probe);
        let (sink, store) = collector();
        let bind = addr.clone();
        tokio::spawn(async move { run_gsi_server(&bind, sink).await });
        tokio::time::sleep(Duration::from_millis(100)).await;

        let body = r#"{"round":{"bomb":"planted"}}"#;
        let mut c = tokio::net::TcpStream::connect(&addr).await.unwrap();
        let req = format!(
            "POST / HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        c.write_all(req.as_bytes()).await.unwrap();
        let mut resp = String::new();
        c.read_to_string(&mut resp).await.unwrap();
        assert!(resp.starts_with("HTTP/1.1 200"));
        let frame = wait_for(&store).await;
        assert!(frame.blink && frame.color.unwrap().r >= 100);
    }

    #[test]
    fn header_end_detection() {
        assert_eq!(find_header_end(b"a\r\n\r\nb"), Some(1));
        assert_eq!(find_header_end(b"abc"), None);
    }
}
