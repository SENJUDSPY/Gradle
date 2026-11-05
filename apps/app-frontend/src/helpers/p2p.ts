import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export type TunnelStatus =
	| { type: 'Connecting' }
	| { type: 'Direct'; data: { remote_addr: string } }
	| { type: 'Relayed'; data: { relay_url: string } }
	| { type: 'Failed'; data: { reason: string } }
	| { type: 'Disconnected' }

export interface ConnectedPeer {
	peer_id: string
	is_direct: boolean
}

export interface P2PHostInfo {
	is_hosting: boolean
	invite_code: string | null
	connected_peers: ConnectedPeer[]
}

export interface P2PJoinInfo {
	is_connected: boolean
	local_port: number | null
	status: TunnelStatus | null
}

export type P2PHostEvent =
	| { type: 'JoinRequest'; data: { peer_id: string } }
	| { type: 'PeerConnected'; data: { peer_id: string; is_direct: boolean } }
	| { type: 'PeerDisconnected'; data: { peer_id: string } }
	| { type: 'InviteRevoked' }
	| { type: 'InviteRegenerated'; data: { code: string } }

export async function startP2PHost(
	serverPort?: number,
	maxPeers?: number,
	requireApproval?: boolean,
): Promise<string> {
	return await invoke('plugin:p2p|p2p_start_host', {
		serverPort: serverPort ?? null,
		maxPeers: maxPeers ?? null,
		requireApproval: requireApproval ?? null,
	})
}

export async function stopP2PHost(): Promise<void> {
	await invoke('plugin:p2p|p2p_stop_host')
}

export async function regenerateInviteCode(): Promise<string> {
	return await invoke('plugin:p2p|p2p_regenerate_invite')
}

export async function revokeInviteCode(): Promise<void> {
	await invoke('plugin:p2p|p2p_revoke_invite')
}

export async function respondJoinRequest(
	peerId: string,
	allow: boolean,
	remember = false,
): Promise<boolean> {
	return await invoke('plugin:p2p|p2p_respond_join_request', {
		peerId,
		allow,
		remember,
	})
}

export async function getP2PHostInfo(): Promise<P2PHostInfo> {
	return await invoke('plugin:p2p|p2p_get_host_info')
}

export async function joinP2P(inviteCode: string): Promise<number> {
	return await invoke('plugin:p2p|p2p_join', { inviteCode })
}

export async function disconnectP2PJoin(): Promise<void> {
	await invoke('plugin:p2p|p2p_disconnect_join')
}

export async function getP2PJoinInfo(): Promise<P2PJoinInfo> {
	return await invoke('plugin:p2p|p2p_get_join_info')
}

export async function onP2PHostEvent(callback: (event: P2PHostEvent) => void): Promise<UnlistenFn> {
	return await listen<P2PHostEvent>('p2p-host-event', (e) => callback(e.payload))
}

export async function onP2PJoinStatus(
	callback: (status: TunnelStatus) => void,
): Promise<UnlistenFn> {
	return await listen<TunnelStatus>('p2p-join-status', (e) => callback(e.payload))
}
