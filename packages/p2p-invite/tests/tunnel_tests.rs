//! Integration tests for P2P Minecraft game tunneling
//!
//! Copyright (c) 2026 Gradle contributors / Gradle authors (MIT License).

use p2p_invite::host::{HostEvent, HostOptions, start_host};
use p2p_invite::joiner::{JoinOptions, join};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Helper that spawns a fake TCP echo server on 127.0.0.1:0
async fn spawn_echo_server() -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                while let Ok(n) = socket.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    if socket.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            });
        }
    });

    addr
}

#[tokio::test]
async fn test_tunnel_data_roundtrip() {
    let echo_addr = spawn_echo_server().await;

    // Start host with auto-approval
    let host_opts = HostOptions {
        require_approval: false,
        ..Default::default()
    };
    let host = start_host(echo_addr, host_opts).await.unwrap();
    let invite_code = host.raw_invite_code().unwrap();

    // Start joiner
    let join_opts = JoinOptions {
        connect_timeout: Duration::from_secs(10),
        ..Default::default()
    };
    let join_handle = join(&invite_code, join_opts).await.unwrap();
    let join_addr = join_handle.local_addr();

    // Connect a simulated game client to the joiner's local TCP port
    let mut client_sock =
        tokio::net::TcpStream::connect(join_addr).await.unwrap();

    // Send data through the tunnel
    let payload = b"GET /minecraft-ping HTTP/1.1\r\n\r\n";
    client_sock.write_all(payload).await.unwrap();

    // Read echoed response
    let mut resp = vec![0u8; payload.len()];
    client_sock.read_exact(&mut resp).await.unwrap();
    assert_eq!(&resp, payload);

    // Send a second packet on the same stream
    let payload2 = b"PING-PONG-26.3-NATIVE-RUST";
    client_sock.write_all(payload2).await.unwrap();

    let mut resp2 = vec![0u8; payload2.len()];
    client_sock.read_exact(&mut resp2).await.unwrap();
    assert_eq!(&resp2, payload2);

    join_handle.stop();
    host.stop();
}

#[tokio::test]
async fn test_revoked_code_rejected() {
    let echo_addr = spawn_echo_server().await;

    let host_opts = HostOptions {
        require_approval: false,
        ..Default::default()
    };
    let host = start_host(echo_addr, host_opts).await.unwrap();
    let invite_code = host.raw_invite_code().unwrap();

    // Revoke the invite before joiner connects
    host.revoke_invite();

    let join_opts = JoinOptions {
        connect_timeout: Duration::from_secs(5),
        ..Default::default()
    };

    let join_res = join(&invite_code, join_opts).await;
    assert!(join_res.is_err(), "Revoked invite code must be rejected");

    host.stop();
}

#[tokio::test]
async fn test_unapproved_peer_rejected() {
    let echo_addr = spawn_echo_server().await;

    // Host requires explicit approval
    let host_opts = HostOptions {
        require_approval: true,
        ..Default::default()
    };
    let host = start_host(echo_addr, host_opts).await.unwrap();
    let invite_code = host.raw_invite_code().unwrap();

    let mut event_rx = host.subscribe_events();

    let host_clone = host.clone();
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let HostEvent::JoinRequest { peer_id } = event {
                // Deny this peer
                host_clone.respond_join_request(&peer_id, false, false);
                break;
            }
        }
    });

    let join_opts = JoinOptions {
        connect_timeout: Duration::from_secs(5),
        ..Default::default()
    };

    let join_res = join(&invite_code, join_opts).await;
    // Joiner connect attempt should fail because host denied peer
    assert!(join_res.is_err());

    host.stop();
}

#[tokio::test]
async fn test_peer_limit_enforced() {
    let echo_addr = spawn_echo_server().await;

    // Max peers = 1
    let host_opts = HostOptions {
        max_peers: 1,
        require_approval: false,
        ..Default::default()
    };
    let host = start_host(echo_addr, host_opts).await.unwrap();
    let invite_code = host.raw_invite_code().unwrap();

    let join_opts = JoinOptions {
        connect_timeout: Duration::from_secs(8),
        ..Default::default()
    };

    // First joiner connects
    let join1 = join(&invite_code, join_opts.clone()).await.unwrap();
    // Wait briefly for peer registration
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(host.connected_peers().len(), 1);

    // Second joiner attempts connection -> should be rejected
    let join2_res = join(&invite_code, join_opts).await;
    assert!(join2_res.is_err());

    join1.stop();
    host.stop();
}
