//! WebTransport client: C0 = first bidirectional stream, C4 = the server's first unidirectional
//! stream (both `[len u16 LE][payload]`), C1/C2/C3 = datagrams `[ch u8][payload]`.

use std::net::SocketAddr;
use std::time::Duration;

use frontline_protocol::Channel;
use tokio::sync::mpsc;
use wtransport::tls::Sha256Digest;
use wtransport::{ClientConfig, Endpoint};

use super::framing::{decode_datagram, encode_datagram, encode_stream_frame, StreamFramer};
use super::{Incoming, Link, Target};

pub async fn connect(t: &Target) -> Result<Link, String> {
    let hash = t
        .cert_hash
        .ok_or_else(|| "WebTransport needs --cert-hash".to_string())?;
    let bind: SocketAddr = match t.bind_ip {
        Some(ip) => SocketAddr::new(ip, 0),
        None => "0.0.0.0:0"
            .parse()
            .map_err(|_| "bind address".to_string())?,
    };
    let cfg = ClientConfig::builder()
        .with_bind_address(bind)
        .with_server_certificate_hashes([Sha256Digest::new(hash)])
        .keep_alive_interval(Some(Duration::from_secs(3)))
        .build();
    let endpoint = Endpoint::client(cfg).map_err(|e| format!("endpoint: {e}"))?;
    let url = format!("https://{}/wt", t.addr);
    let work = async {
        let conn = endpoint
            .connect(&url)
            .await
            .map_err(|e| format!("connect: {e}"))?;
        let (c0_tx, c0_rx) = conn
            .open_bi()
            .await
            .map_err(|e| format!("open_bi: {e}"))?
            .await
            .map_err(|e| format!("open_bi: {e}"))?;
        Ok::<_, String>((conn, c0_tx, c0_rx))
    };
    let (conn, mut c0_tx, mut c0_rx) = tokio::time::timeout(t.connect_timeout, work)
        .await
        .map_err(|_| "connect timeout".to_string())??;

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<(Channel, Vec<u8>)>();
    let (in_tx, in_rx) = mpsc::unbounded_channel();
    let mut tasks = Vec::new();

    // writer: C0 on the stream, everything else as datagrams
    let wconn = conn.clone();
    tasks.push(tokio::spawn(async move {
        while let Some((ch, payload)) = out_rx.recv().await {
            match ch {
                Channel::C0 => {
                    let Some(f) = encode_stream_frame(&payload) else {
                        continue;
                    };
                    if c0_tx.write_all(&f).await.is_err() {
                        return;
                    }
                }
                _ => {
                    if wconn.send_datagram(encode_datagram(ch, &payload)).is_err() {
                        return;
                    }
                }
            }
        }
    }));
    // C0 reader
    let tx = in_tx.clone();
    tasks.push(tokio::spawn(async move {
        let mut f = StreamFramer::default();
        let mut buf = vec![0u8; 4096];
        while let Ok(Some(n)) = c0_rx.read(&mut buf).await {
            f.push(&buf[..n]);
            while let Some(p) = f.next_frame() {
                if tx.send(Incoming::Frame(Channel::C0, p)).is_err() {
                    return;
                }
            }
        }
    }));
    // C4 reader (server's first uni stream)
    let tx = in_tx.clone();
    let c = conn.clone();
    tasks.push(tokio::spawn(async move {
        let Ok(mut s) = c.accept_uni().await else {
            return;
        };
        let mut f = StreamFramer::default();
        let mut buf = vec![0u8; 4096];
        while let Ok(Some(n)) = s.read(&mut buf).await {
            f.push(&buf[..n]);
            while let Some(p) = f.next_frame() {
                if tx.send(Incoming::Frame(Channel::C4, p)).is_err() {
                    return;
                }
            }
        }
    }));
    // datagrams
    let tx = in_tx.clone();
    let c = conn.clone();
    tasks.push(tokio::spawn(async move {
        while let Ok(d) = c.receive_datagram().await {
            if let Some((ch, p)) = decode_datagram(&d.payload()) {
                if tx.send(Incoming::Frame(ch, p.to_vec())).is_err() {
                    return;
                }
            }
        }
    }));
    // closure; also keeps the endpoint alive for the lifetime of the link
    let c = conn.clone();
    tasks.push(tokio::spawn(async move {
        let reason = c.closed().await;
        drop(endpoint);
        let _ = in_tx.send(Incoming::Closed(format!("{reason}")));
    }));
    Ok(Link::new(out_tx, in_rx, tasks))
}
