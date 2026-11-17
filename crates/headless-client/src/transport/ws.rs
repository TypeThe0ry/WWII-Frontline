//! WebSocket client: binary messages carry the `[ch u8][len u16 LE][payload]` envelope.

use std::net::SocketAddr;

use frontline_protocol::framing::{decode_ws_frame, encode_ws_frame};
use frontline_protocol::Channel;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpSocket;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::{Incoming, Link, Target};

pub async fn connect(t: &Target) -> Result<Link, String> {
    let work = async {
        let addr: SocketAddr = tokio::net::lookup_host(&t.addr)
            .await
            .map_err(|e| format!("resolve {}: {e}", t.addr))?
            .find(|a| a.is_ipv4() || t.bind_ip.is_none())
            .ok_or_else(|| format!("no address for {}", t.addr))?;
        let sock = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .map_err(|e| e.to_string())?;
        if let Some(ip) = t.bind_ip {
            sock.bind(SocketAddr::new(ip, 0)).map_err(|e| format!("bind {ip}: {e}"))?;
        }
        let stream = sock.connect(addr).await.map_err(|e| format!("tcp connect: {e}"))?;
        let _ = stream.set_nodelay(true);
        let (ws, _) = tokio_tungstenite::client_async(format!("ws://{}", t.addr), stream)
            .await
            .map_err(|e| format!("ws handshake: {e}"))?;
        Ok::<_, String>(ws)
    };
    let ws = tokio::time::timeout(t.connect_timeout, work)
        .await
        .map_err(|_| "connect timeout".to_string())??;
    let (mut sink, mut stream) = ws.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<(Channel, Vec<u8>)>();
    let (in_tx, in_rx) = mpsc::unbounded_channel();
    let writer = tokio::spawn(async move {
        while let Some((ch, payload)) = out_rx.recv().await {
            let Ok(frame) = encode_ws_frame(ch, &payload) else {
                continue;
            };
            if sink.send(Message::Binary(frame.into())).await.is_err() {
                return;
            }
        }
        let _ = sink.close().await;
    });
    let reader = tokio::spawn(async move {
        let reason = loop {
            match stream.next().await {
                Some(Ok(Message::Binary(b))) => {
                    if let Ok((ch, p)) = decode_ws_frame(&b) {
                        if in_tx.send(Incoming::Frame(ch, p.to_vec())).is_err() {
                            return;
                        }
                    }
                }
                Some(Ok(Message::Close(c))) => break format!("closed by server: {c:?}"),
                Some(Ok(_)) => {}
                Some(Err(e)) => break format!("ws error: {e}"),
                None => break "stream ended".to_string(),
            }
        };
        let _ = in_tx.send(Incoming::Closed(reason));
    });
    Ok(Link::new(out_tx, in_rx, vec![writer, reader]))
}
