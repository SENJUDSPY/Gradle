//! Host engine for P2P Minecraft game tunneling
//!
//! Copyright (c) 2026 AstralRinth contributors / Gradle authors (MIT License).

use crate::invite::InviteCode;
use crate::protocol::{
    ALPN, DEFAULT_IDLE_TIMEOUT, DEFAULT_MAX_PEERS,
    DEFAULT_MAX_STREAMS_PER_PEER, HANDSHAKE_RESP_OK, HANDSHAKE_RESP_REJECTED,
    HANDSHAKE_RESP_REVOKED, HandshakeKind, StreamHandshake,
};
use anyhow::{Context, Result, bail};
use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointId};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot};

#[derive(Debug, Clone)]
pub struct HostOptions {
    pub max_peers: usize,
    pub max_streams_per_peer: usize,
    pub idle_timeout: Duration,
    pub require_approval: bool,
    pub initial_allowlist: HashSet<String>,
}

impl Default for HostOptions {
    fn default() -> Self {
        Self {
            max_peers: DEFAULT_MAX_PEERS,
            max_streams_per_peer: DEFAULT_MAX_STREAMS_PER_PEER,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            require_approval: true,
            initial_allowlist: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectedPeer {
    pub peer_id: String,
    pub is_direct: bool,
}

#[derive(Debug, Clone)]
pub enum HostEvent {
    JoinRequest { peer_id: String },
    PeerConnected { peer_id: String, is_direct: bool },
    PeerDisconnected { peer_id: String },
    InviteRegenerated { code: InviteCode },
    InviteRevoked,
    Stopped,
}

struct PeerState {
    conn: iroh::endpoint::Connection,
    active_streams: Arc<AtomicUsize>,
}

struct HostInner {
    local_server_addr: SocketAddr,
    endpoint: Endpoint,
    current_token: String,
    is_revoked: bool,
    invite_code: Option<InviteCode>,
    options: HostOptions,
    allowed_peers: HashSet<String>,
    denied_peers: HashSet<String>,
    pending_approvals: HashMap<String, oneshot::Sender<bool>>,
    connected_peers: HashMap<String, PeerState>,
    stopped: bool,
}

#[derive(Clone)]
pub struct HostHandle {
    inner: Arc<RwLock<HostInner>>,
    event_tx: broadcast::Sender<HostEvent>,
}

impl HostHandle {
    pub fn invite_code(&self) -> Option<InviteCode> {
        let lock = self.inner.read();
        if lock.is_revoked || lock.stopped {
            None
        } else {
            lock.invite_code.clone()
        }
    }

    pub fn raw_invite_code(&self) -> Option<String> {
        self.invite_code().map(|c| c.raw_code().to_string())
    }

    pub fn local_server_addr(&self) -> SocketAddr {
        self.inner.read().local_server_addr
    }

    pub fn endpoint_id(&self) -> EndpointId {
        self.inner.read().endpoint.id()
    }

    pub fn connected_peers(&self) -> Vec<String> {
        self.inner.read().connected_peers.keys().cloned().collect()
    }

    pub fn connected_peer_details(&self) -> Vec<ConnectedPeer> {
        let lock = self.inner.read();
        lock.connected_peers
            .iter()
            .map(|(id, state)| {
                let is_direct = state
                    .conn
                    .paths()
                    .iter()
                    .any(|p| p.is_selected() && p.is_ip());
                ConnectedPeer {
                    peer_id: id.clone(),
                    is_direct,
                }
            })
            .collect()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<HostEvent> {
        self.event_tx.subscribe()
    }

    pub fn revoke_invite(&self) {
        let mut lock = self.inner.write();
        lock.is_revoked = true;
        lock.invite_code = None;

        // Disconnect all currently connected peers on revocation
        for (id, peer) in lock.connected_peers.drain() {
            peer.conn.close(1u32.into(), b"invite revoked");
            let _ = self
                .event_tx
                .send(HostEvent::PeerDisconnected { peer_id: id });
        }

        let _ = self.event_tx.send(HostEvent::InviteRevoked);
    }

    pub fn regenerate_invite(&self) -> Result<InviteCode> {
        let mut lock = self.inner.write();
        if lock.stopped {
            bail!("Host is stopped");
        }

        let new_token = InviteCode::generate_random_token();
        let addr = lock.endpoint.addr();
        let code = InviteCode::new(addr, &new_token)?;

        lock.current_token = new_token;
        lock.is_revoked = false;
        lock.invite_code = Some(code.clone());

        let _ = self
            .event_tx
            .send(HostEvent::InviteRegenerated { code: code.clone() });

        Ok(code)
    }

    pub fn respond_join_request(
        &self,
        peer_id: &str,
        allow: bool,
        remember: bool,
    ) -> bool {
        let mut lock = self.inner.write();
        if allow {
            lock.allowed_peers.insert(peer_id.to_string());
            if !remember {
                // Kept for current session
            }
        } else {
            lock.denied_peers.insert(peer_id.to_string());
        }

        if let Some(sender) = lock.pending_approvals.remove(peer_id) {
            let _ = sender.send(allow);
            true
        } else {
            false
        }
    }

    pub fn disconnect_peer(&self, peer_id: &str) -> bool {
        let mut lock = self.inner.write();
        if let Some(peer) = lock.connected_peers.remove(peer_id) {
            peer.conn.close(0u32.into(), b"disconnected by host");
            let _ = self.event_tx.send(HostEvent::PeerDisconnected {
                peer_id: peer_id.to_string(),
            });
            true
        } else {
            false
        }
    }

    pub fn stop(&self) {
        let (endpoint, peers) = {
            let mut lock = self.inner.write();
            if lock.stopped {
                return;
            }
            lock.stopped = true;
            let peers = lock
                .connected_peers
                .drain()
                .map(|(id, p)| (id, p.conn))
                .collect::<Vec<_>>();
            (lock.endpoint.clone(), peers)
        };

        for (id, conn) in peers {
            conn.close(0u32.into(), b"host stopped");
            let _ = self
                .event_tx
                .send(HostEvent::PeerDisconnected { peer_id: id });
        }

        tokio::spawn(async move {
            endpoint.close().await;
        });

        let _ = self.event_tx.send(HostEvent::Stopped);
    }
}

pub async fn start_host(
    local_server_addr: SocketAddr,
    options: HostOptions,
) -> Result<HostHandle> {
    let endpoint = Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .context("binding host Iroh endpoint")?;

    let _ =
        tokio::time::timeout(Duration::from_secs(3), endpoint.online()).await;

    let initial_token = InviteCode::generate_random_token();
    let addr = endpoint.addr();
    let invite_code = InviteCode::new(addr, &initial_token)?;

    let (event_tx, _) = broadcast::channel(128);

    let inner = Arc::new(RwLock::new(HostInner {
        local_server_addr,
        endpoint: endpoint.clone(),
        current_token: initial_token,
        is_revoked: false,
        invite_code: Some(invite_code),
        allowed_peers: options.initial_allowlist.clone(),
        denied_peers: HashSet::new(),
        pending_approvals: HashMap::new(),
        connected_peers: HashMap::new(),
        stopped: false,
        options,
    }));

    let handle = HostHandle {
        inner: inner.clone(),
        event_tx: event_tx.clone(),
    };

    let host_handle_clone = handle.clone();
    tokio::spawn(async move {
        run_host_accept_loop(host_handle_clone).await;
    });

    Ok(handle)
}

async fn run_host_accept_loop(handle: HostHandle) {
    let endpoint = handle.inner.read().endpoint.clone();

    while let Some(incoming) = endpoint.accept().await {
        if handle.inner.read().stopped {
            break;
        }

        let handle_clone = handle.clone();
        tokio::spawn(async move {
            if let Err(err) =
                handle_incoming_connection(incoming, handle_clone).await
            {
                tracing::debug!("Host incoming connection ended: {err:#}");
            }
        });
    }
}

async fn handle_incoming_connection(
    incoming: iroh::endpoint::Incoming,
    handle: HostHandle,
) -> Result<()> {
    let conn = incoming.await.context("completing peer handshake")?;
    let peer_id = conn.remote_id().to_string();

    // Check host approval and limits
    {
        let (max_peers, require_approval, is_allowed, is_denied) = {
            let lock = handle.inner.read();
            if lock.stopped || lock.is_revoked {
                conn.close(1u32.into(), b"host unavailable");
                return Ok(());
            }

            if lock.connected_peers.len() >= lock.options.max_peers {
                conn.close(2u32.into(), b"max peers limit reached");
                bail!("Maximum peers reached ({})", lock.options.max_peers);
            }

            let is_allowed = lock.allowed_peers.contains(&peer_id);
            let is_denied = lock.denied_peers.contains(&peer_id);
            (
                lock.options.max_peers,
                lock.options.require_approval,
                is_allowed,
                is_denied,
            )
        };

        if is_denied {
            conn.close(1u32.into(), b"peer denied");
            bail!("Peer {peer_id} is denied");
        }

        if require_approval && !is_allowed {
            let (tx, rx) = oneshot::channel();
            {
                let mut lock = handle.inner.write();
                lock.pending_approvals.insert(peer_id.clone(), tx);
            }

            let _ = handle.event_tx.send(HostEvent::JoinRequest {
                peer_id: peer_id.clone(),
            });

            let approved =
                match tokio::time::timeout(Duration::from_secs(30), rx).await {
                    Ok(Ok(val)) => val,
                    _ => false,
                };

            {
                let mut lock = handle.inner.write();
                lock.pending_approvals.remove(&peer_id);
            }

            if !approved {
                conn.close(1u32.into(), b"connection rejected by host");
                bail!("Peer {peer_id} not approved by host");
            }
        }

        let mut lock = handle.inner.write();
        if lock.connected_peers.len() >= max_peers {
            conn.close(2u32.into(), b"max peers limit reached");
            bail!("Max peers reached");
        }

        let is_direct =
            conn.paths().iter().any(|p| p.is_selected() && p.is_ip());
        let active_streams = Arc::new(AtomicUsize::new(0));

        lock.connected_peers.insert(
            peer_id.clone(),
            PeerState {
                conn: conn.clone(),
                active_streams: active_streams.clone(),
            },
        );

        let _ = handle.event_tx.send(HostEvent::PeerConnected {
            peer_id: peer_id.clone(),
            is_direct,
        });
    }

    let active_streams = handle
        .inner
        .read()
        .connected_peers
        .get(&peer_id)
        .map(|p| p.active_streams.clone())
        .unwrap_or_else(|| Arc::new(AtomicUsize::new(0)));

    loop {
        let (send, recv) = match conn.accept_bi().await {
            Ok(bi) => bi,
            Err(_) => break,
        };

        let handle_clone = handle.clone();
        let peer_id_clone = peer_id.clone();
        let streams_clone = active_streams.clone();

        tokio::spawn(async move {
            let max_streams =
                handle_clone.inner.read().options.max_streams_per_peer;
            let current = streams_clone.fetch_add(1, Ordering::SeqCst);
            if current >= max_streams {
                streams_clone.fetch_sub(1, Ordering::SeqCst);
                let mut s = send;
                let _ = s.reset(2u32.into());
                return;
            }

            let res = handle_host_tunnel_stream(
                send,
                recv,
                handle_clone,
                &peer_id_clone,
            )
            .await;
            streams_clone.fetch_sub(1, Ordering::SeqCst);

            if let Err(e) = res {
                tracing::debug!("Host stream proxy ended: {e:#}");
            }
        });
    }

    {
        let mut lock = handle.inner.write();
        lock.connected_peers.remove(&peer_id);
    }
    let _ = handle
        .event_tx
        .send(HostEvent::PeerDisconnected { peer_id });

    Ok(())
}

async fn handle_host_tunnel_stream(
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
    handle: HostHandle,
    peer_id: &str,
) -> Result<()> {
    // 1. Read joiner handshake
    let handshake = StreamHandshake::read_from(&mut recv).await?;

    // 2. Verify token matches active host token and host is not revoked
    let (is_valid, server_addr, idle_timeout) = {
        let lock = handle.inner.read();
        let valid = !lock.is_revoked
            && !lock.stopped
            && lock.current_token == handshake.token;
        (valid, lock.local_server_addr, lock.options.idle_timeout)
    };

    if !is_valid {
        send.write_u8(HANDSHAKE_RESP_REVOKED).await?;
        send.finish()?;
        bail!("Invalid or revoked invite token from {peer_id}");
    }

    // Handle Verify handshake
    if handshake.kind == HandshakeKind::Verify {
        send.write_u8(HANDSHAKE_RESP_OK).await?;
        send.finish()?;
        return Ok(());
    }

    // Send OK response to joiner for Tunnel
    send.write_u8(HANDSHAKE_RESP_OK).await?;
    send.flush().await?;

    // 3. Connect to the local Minecraft server TCP socket
    let mut tcp_stream = match TcpStream::connect(server_addr).await {
        Ok(s) => s,
        Err(e) => {
            send.write_u8(HANDSHAKE_RESP_REJECTED).await?;
            send.finish()?;
            bail!("Failed to connect to local server at {server_addr}: {e}");
        }
    };

    // 4. Bidirectional byte tunnel between TCP and QUIC streams
    let (mut tcp_read, mut tcp_write) = tcp_stream.split();

    let client_to_server = tokio::io::copy(&mut recv, &mut tcp_write);
    let server_to_client = tokio::io::copy(&mut tcp_read, &mut send);

    tokio::select! {
        _ = tokio::time::sleep(idle_timeout) => {
            let _ = send.finish();
        }
        res1 = client_to_server => {
            let _ = res1;
            let _ = tcp_write.shutdown().await;
        }
        res2 = server_to_client => {
            let _ = res2;
            let _ = send.finish();
        }
    }

    Ok(())
}
