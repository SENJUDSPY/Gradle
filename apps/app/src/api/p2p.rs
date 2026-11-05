use crate::api::Result;
use p2p_invite::host::{ConnectedPeer, HostEvent, HostHandle, HostOptions, start_host};
use p2p_invite::joiner::{JoinHandle, JoinOptions, TunnelStatus, join};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tauri::{Emitter, Manager, Runtime};

#[derive(Default)]
pub struct P2PState {
	pub host: parking_lot::Mutex<Option<HostHandle>>,
	pub joiner: parking_lot::Mutex<Option<JoinHandle>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PHostInfo {
	pub is_hosting: bool,
	pub invite_code: Option<String>,
	pub connected_peers: Vec<ConnectedPeer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PJoinInfo {
	pub is_connected: bool,
	pub local_port: Option<u16>,
	pub status: Option<TunnelStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum P2PHostEventPayload {
	JoinRequest { peer_id: String },
	PeerConnected { peer_id: String, is_direct: bool },
	PeerDisconnected { peer_id: String },
	InviteRevoked,
	InviteRegenerated { code: String },
}

#[tauri::command]
pub async fn p2p_start_host(
	app: tauri::AppHandle,
	state: tauri::State<'_, P2PState>,
	server_port: Option<u16>,
	max_peers: Option<usize>,
	require_approval: Option<bool>,
) -> Result<String> {
	let port = server_port.unwrap_or(25565);
	let addr = SocketAddr::from(([127, 0, 0, 1], port));
	let mut options = HostOptions::default();
	if let Some(m) = max_peers {
		options.max_peers = m;
	}
	if let Some(r) = require_approval {
		options.require_approval = r;
	}

	let host = start_host(addr, options).await?;
	let code = host
		.raw_invite_code()
		.ok_or_else(|| anyhow::anyhow!("Failed to generate invite code"))?;

	let mut rx = host.subscribe_events();
	let app_clone = app.clone();
	tokio::spawn(async move {
		while let Ok(evt) = rx.recv().await {
			let payload = match evt {
				HostEvent::JoinRequest { peer_id } => P2PHostEventPayload::JoinRequest { peer_id },
				HostEvent::PeerConnected { peer_id, is_direct } => {
					P2PHostEventPayload::PeerConnected { peer_id, is_direct }
				}
				HostEvent::PeerDisconnected { peer_id } => {
					P2PHostEventPayload::PeerDisconnected { peer_id }
				}
				HostEvent::InviteRevoked => P2PHostEventPayload::InviteRevoked,
				HostEvent::InviteRegenerated { code } => {
					P2PHostEventPayload::InviteRegenerated {
						code: code.raw_code().to_string(),
					}
				}
			};
			let _ = app_clone.emit("p2p-host-event", payload);
		}
	});

	let mut lock = state.host.lock();
	if let Some(old) = lock.take() {
		old.stop();
	}
	*lock = Some(host);

	Ok(code)
}

#[tauri::command]
pub async fn p2p_stop_host(state: tauri::State<'_, P2PState>) -> Result<()> {
	let mut lock = state.host.lock();
	if let Some(h) = lock.take() {
		h.stop();
	}
	Ok(())
}

#[tauri::command]
pub async fn p2p_regenerate_invite(state: tauri::State<'_, P2PState>) -> Result<String> {
	let lock = state.host.lock();
	if let Some(ref h) = *lock {
		let code = h.regenerate_invite()?;
		Ok(code.raw_code().to_string())
	} else {
		anyhow::bail!("No active P2P host running")
	}
}

#[tauri::command]
pub async fn p2p_revoke_invite(state: tauri::State<'_, P2PState>) -> Result<()> {
	let lock = state.host.lock();
	if let Some(ref h) = *lock {
		h.revoke_invite();
		Ok(())
	} else {
		anyhow::bail!("No active P2P host running")
	}
}

#[tauri::command]
pub async fn p2p_respond_join_request(
	state: tauri::State<'_, P2PState>,
	peer_id: String,
	allow: bool,
	remember: bool,
) -> Result<bool> {
	let lock = state.host.lock();
	if let Some(ref h) = *lock {
		Ok(h.respond_join_request(&peer_id, allow, remember))
	} else {
		anyhow::bail!("No active P2P host running")
	}
}

#[tauri::command]
pub async fn p2p_get_host_info(state: tauri::State<'_, P2PState>) -> Result<P2PHostInfo> {
	let lock = state.host.lock();
	if let Some(ref h) = *lock {
		Ok(P2PHostInfo {
			is_hosting: true,
			invite_code: h.raw_invite_code(),
			connected_peers: h.connected_peer_details(),
		})
	} else {
		Ok(P2PHostInfo {
			is_hosting: false,
			invite_code: None,
			connected_peers: Vec::new(),
		})
	}
}

#[tauri::command]
pub async fn p2p_join(
	app: tauri::AppHandle,
	state: tauri::State<'_, P2PState>,
	invite_code: String,
) -> Result<u16> {
	let trimmed = invite_code.trim();
	let handle = join(trimmed, JoinOptions::default()).await?;
	let port = handle.local_addr().port();

	let mut status_rx = handle.subscribe_status();
	let app_clone = app.clone();
	tokio::spawn(async move {
		while status_rx.changed().await.is_ok() {
			let status = status_rx.borrow().clone();
			let _ = app_clone.emit("p2p-join-status", &status);
		}
	});

	let mut lock = state.joiner.lock();
	if let Some(old) = lock.take() {
		old.stop();
	}
	*lock = Some(handle);

	Ok(port)
}

#[tauri::command]
pub async fn p2p_disconnect_join(state: tauri::State<'_, P2PState>) -> Result<()> {
	let mut lock = state.joiner.lock();
	if let Some(j) = lock.take() {
		j.stop();
	}
	Ok(())
}

#[tauri::command]
pub async fn p2p_get_join_info(state: tauri::State<'_, P2PState>) -> Result<P2PJoinInfo> {
	let lock = state.joiner.lock();
	if let Some(ref j) = *lock {
		Ok(P2PJoinInfo {
			is_connected: true,
			local_port: Some(j.local_addr().port()),
			status: Some(j.current_status()),
		})
	} else {
		Ok(P2PJoinInfo {
			is_connected: false,
			local_port: None,
			status: None,
		})
	}
}

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("p2p")
		.setup(|app, _| {
			app.manage(P2PState::default());
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			p2p_start_host,
			p2p_stop_host,
			p2p_regenerate_invite,
			p2p_revoke_invite,
			p2p_respond_join_request,
			p2p_get_host_info,
			p2p_join,
			p2p_disconnect_join,
			p2p_get_join_info,
		])
		.build()
}
