//! Joiner engine for P2P Minecraft game tunneling
//!
//! Copyright (c) 2026 Gradle contributors / Gradle authors (MIT License).

use crate::invite::InviteCode;
use crate::protocol::{
    ALPN, DEFAULT_CONNECT_TIMEOUT, HANDSHAKE_RESP_OK, HANDSHAKE_RESP_REVOKED,
    HandshakeKind, StreamHandshake,
};
use anyhow::{Context, Result, bail};
use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointId};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TunnelStatus {
    Connecting,
    Direct { remote_addr: String },
    Relayed { relay_url: String },
    Failed { reason: String },
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct JoinOptions {
    pub local_bind_addr: Option<SocketAddr>,
    pub connect_timeout: Duration,
}

impl Default for JoinOptions {
    fn default() -> Self {
        Self {
            local_bind_addr: None,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
        }
    }
}

#[derive(Clone)]
pub struct JoinHandle {
    local_addr: SocketAddr,
    host_endpoint_id: EndpointId,
    status_rx: watch::Receiver<TunnelStatus>,
    stopped: Arc<AtomicBool>,
    endpoint: Endpoint,
}

impl JoinHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn host_endpoint_id(&self) -> EndpointId {
        self.host_endpoint_id
    }

    pub fn current_status(&self) -> TunnelStatus {
        self.status_rx.borrow().clone()
    }

    pub fn subscribe_status(&self) -> watch::Receiver<TunnelStatus> {
        self.status_rx.clone()
    }

    pub fn stop(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            let ep = self.endpoint.clone();
            tokio::spawn(async move {
                ep.close().await;
            });
        }
    }
}

pub async fn join(
    invite_code_str: &str,
    options: JoinOptions,
) -> Result<JoinHandle> {
    let invite = InviteCode::decode(invite_code_str)
        .context("decoding peer invite code")?;

    let bind_target = options
        .local_bind_addr
        .unwrap_or_else(|| "127.0.0.1:0".parse().unwrap());

    let tcp_listener =
        TcpListener::bind(bind_target).await.with_context(|| {
            format!("binding local TCP listener on {bind_target}")
        })?;
    let local_addr = tcp_listener.local_addr()?;

    let (status_tx, status_rx) = watch::channel(TunnelStatus::Connecting);

    let endpoint = Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .context("binding client Iroh endpoint")?;

    let host_addr = invite.addr().clone();
    let host_id = invite.endpoint_id();
    let token = invite.token().to_string();

    // Connect to host with timeout
    let conn = match tokio::time::timeout(
        options.connect_timeout,
        endpoint.connect(host_addr, ALPN),
    )
    .await
    {
        Ok(res) => res.context("completing host handshake")?,
        Err(_) => {
            let _ = status_tx.send(TunnelStatus::Failed {
                reason: "Connection to host timed out".to_string(),
            });
            endpoint.close().await;
            bail!("Connection to host timed out");
        }
    };

    // Perform immediate verify stream handshake to validate token and host approval
    let (mut quic_send, mut quic_recv) =
        match tokio::time::timeout(options.connect_timeout, conn.open_bi())
            .await
        {
            Ok(Ok(bi)) => bi,
            Ok(Err(e)) => {
                let _ = status_tx.send(TunnelStatus::Failed {
                    reason: format!("Peer connection rejected by host: {e}"),
                });
                endpoint.close().await;
                bail!("Peer connection rejected by host: {e}");
            }
            Err(_) => {
                let _ = status_tx.send(TunnelStatus::Failed {
                    reason: "Host verification timed out".to_string(),
                });
                endpoint.close().await;
                bail!("Host verification timed out");
            }
        };

    let verify_handshake = StreamHandshake::new(HandshakeKind::Verify, &token);
    verify_handshake.write_to(&mut quic_send).await?;

    let resp = match quic_recv.read_u8().await {
        Ok(b) => b,
        Err(e) => {
            let _ = status_tx.send(TunnelStatus::Failed {
                reason: format!(
                    "Host closed connection during verification: {e}"
                ),
            });
            endpoint.close().await;
            bail!("Host closed connection during verification: {e}");
        }
    };

    if resp == HANDSHAKE_RESP_REVOKED {
        let err_msg = "Invite code was revoked by host".to_string();
        let _ = status_tx.send(TunnelStatus::Failed {
            reason: err_msg.clone(),
        });
        endpoint.close().await;
        bail!(err_msg);
    } else if resp != HANDSHAKE_RESP_OK {
        let err_msg = format!("Connection rejected by host (code {resp})");
        let _ = status_tx.send(TunnelStatus::Failed {
            reason: err_msg.clone(),
        });
        endpoint.close().await;
        bail!(err_msg);
    }

    let _ = quic_send.finish();

    let stopped = Arc::new(AtomicBool::new(false));

    // Spawn path monitoring loop
    let conn_monitor = conn.clone();
    let status_tx_monitor = status_tx.clone();
    let stopped_monitor = stopped.clone();

    tokio::spawn(async move {
        run_path_monitor(conn_monitor, status_tx_monitor, stopped_monitor)
            .await;
    });

    // Spawn TCP accept proxy loop
    let conn_proxy = conn.clone();
    let stopped_proxy = stopped.clone();
    let status_tx_proxy = status_tx.clone();

    tokio::spawn(async move {
        if let Err(e) = run_joiner_proxy_loop(
            tcp_listener,
            conn_proxy,
            token,
            stopped_proxy,
            status_tx_proxy,
        )
        .await
        {
            tracing::debug!("Joiner proxy loop terminated: {e:#}");
        }
    });

    Ok(JoinHandle {
        local_addr,
        host_endpoint_id: host_id,
        status_rx,
        stopped,
        endpoint,
    })
}

async fn run_path_monitor(
    conn: iroh::endpoint::Connection,
    status_tx: watch::Sender<TunnelStatus>,
    stopped: Arc<AtomicBool>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(400));

    while !stopped.load(Ordering::SeqCst) {
        interval.tick().await;

        let mut direct_addr = None;
        let mut relay_url = None;

        for p in &conn.paths() {
            if p.is_selected() {
                if p.is_ip() {
                    direct_addr = Some(p.remote_addr().to_string());
                    break;
                } else if p.is_relay() {
                    relay_url = Some(p.remote_addr().to_string());
                }
            }
        }

        let new_status = if let Some(addr) = direct_addr {
            TunnelStatus::Direct { remote_addr: addr }
        } else if let Some(url) = relay_url {
            TunnelStatus::Relayed { relay_url: url }
        } else {
            TunnelStatus::Connecting
        };

        if *status_tx.borrow() != new_status {
            let _ = status_tx.send(new_status);
        }
    }

    let _ = status_tx.send(TunnelStatus::Disconnected);
}

async fn run_joiner_proxy_loop(
    tcp_listener: TcpListener,
    conn: iroh::endpoint::Connection,
    token: String,
    stopped: Arc<AtomicBool>,
    status_tx: watch::Sender<TunnelStatus>,
) -> Result<()> {
    while !stopped.load(Ordering::SeqCst) {
        let (mut tcp_socket, _) = match tcp_listener.accept().await {
            Ok(res) => res,
            Err(_) => break,
        };

        let conn_clone = conn.clone();
        let token_clone = token.clone();
        let status_tx_clone = status_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_joiner_stream(
                &mut tcp_socket,
                conn_clone,
                &token_clone,
                &status_tx_clone,
            )
            .await
            {
                tracing::debug!("Joiner game stream ended: {e:#}");
            }
        });
    }

    Ok(())
}

async fn handle_joiner_stream(
    tcp_socket: &mut TcpStream,
    conn: iroh::endpoint::Connection,
    token: &str,
    status_tx: &watch::Sender<TunnelStatus>,
) -> Result<()> {
    // 1. Open bidirectional stream to host
    let (mut quic_send, mut quic_recv) =
        conn.open_bi().await.context("opening bi-stream to host")?;

    // 2. Transmit handshake token with Tunnel kind
    let handshake = StreamHandshake::new(HandshakeKind::Tunnel, token);
    handshake.write_to(&mut quic_send).await?;

    // 3. Receive host response byte
    let resp = quic_recv
        .read_u8()
        .await
        .context("reading handshake response")?;
    if resp == HANDSHAKE_RESP_REVOKED {
        let err_msg = "Invite code was revoked by host".to_string();
        let _ = status_tx.send(TunnelStatus::Failed {
            reason: err_msg.clone(),
        });
        bail!(err_msg);
    } else if resp != HANDSHAKE_RESP_OK {
        bail!("Connection rejected by host (code {resp})");
    }

    // 4. Bidirectional tunnel between TCP client and QUIC stream
    let (mut tcp_read, mut tcp_write) = tcp_socket.split();

    let client_to_server = tokio::io::copy(&mut tcp_read, &mut quic_send);
    let server_to_client = tokio::io::copy(&mut quic_recv, &mut tcp_write);

    tokio::select! {
        res1 = client_to_server => {
            let _ = res1;
            let _ = quic_send.finish();
        }
        res2 = server_to_client => {
            let _ = res2;
            let _ = tcp_write.shutdown().await;
        }
    }

    Ok(())
}
