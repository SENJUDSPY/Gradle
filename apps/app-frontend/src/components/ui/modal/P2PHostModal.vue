<template>
	<NewModal ref="modalRef" :header="formatMessage(messages.title)" max-width="580px" width="580px">
		<div class="flex w-full flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.subtitle) }}
			</p>

			<!-- Error banner -->
			<div v-if="error" class="rounded-xl bg-red-500/10 p-3 text-xs text-red-400">
				{{ error }}
			</div>

			<!-- Pending Join Requests (Host Approval) -->
			<div
				v-if="pendingRequests.length > 0"
				class="flex flex-col gap-2 rounded-xl border border-solid border-orange/30 bg-orange/10 p-3.5"
			>
				<div class="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-orange">
					<ShieldCheckIcon class="size-4 shrink-0" />
					<span>{{ formatMessage(messages.joinRequestPrompt) }}</span>
				</div>
				<div
					v-for="req in pendingRequests"
					:key="req.peer_id"
					class="flex flex-col gap-2 rounded-lg bg-surface-2 p-2.5"
				>
					<div class="flex items-center justify-between">
						<span class="font-mono text-xs text-contrast"> {{ req.peer_id.slice(0, 16) }}... </span>
						<div class="flex items-center gap-2">
							<Button
								size="sm"
								type="colored"
								color="brand"
								@click="handleRespondRequest(req.peer_id, true)"
							>
								<CheckIcon class="size-3.5" />
								{{ formatMessage(messages.allow) }}
							</Button>
							<Button size="sm" type="outlined" @click="handleRespondRequest(req.peer_id, false)">
								<XIcon class="size-3.5" />
								{{ formatMessage(messages.deny) }}
							</Button>
						</div>
					</div>
					<label class="flex cursor-pointer items-center gap-2 text-xs text-secondary">
						<input v-model="rememberSession" type="checkbox" class="rounded border-surface-5" />
						<span>{{ formatMessage(messages.rememberApproval) }}</span>
					</label>
				</div>
			</div>

			<!-- View 1: Not hosting yet -->
			<div v-if="!isHosting" class="flex flex-col gap-4">
				<div class="rounded-xl border border-solid border-surface-5 bg-surface-2 p-4">
					<div class="grid grid-cols-2 gap-3">
						<label class="flex flex-col gap-1.5 text-xs text-secondary">
							<span class="font-semibold text-contrast">{{ formatMessage(messages.port) }}</span>
							<Input v-model.number="serverPort" type="number" :min="1024" :max="65535" />
						</label>

						<label class="flex flex-col gap-1.5 text-xs text-secondary">
							<span class="font-semibold text-contrast">{{
								formatMessage(messages.maxPeers)
							}}</span>
							<Input v-model.number="maxPeers" type="number" :min="1" :max="64" />
						</label>
					</div>

					<div
						class="mt-3 flex items-center justify-between border-t border-solid border-surface-4 pt-3"
					>
						<div class="flex flex-col">
							<span class="text-xs font-semibold text-contrast">
								{{ formatMessage(messages.requireApprovalLabel) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.requireApprovalDesc) }}
							</span>
						</div>
						<input
							v-model="requireApproval"
							type="checkbox"
							class="size-4 cursor-pointer rounded border-surface-5"
						/>
					</div>
				</div>

				<div class="flex justify-end gap-2">
					<Button type="quiet" @click="hide">
						{{ formatMessage(commonMessages.cancelButton) }}
					</Button>
					<Button type="colored" color="brand" :disabled="loading" @click="handleStartHost">
						<ShareIcon class="size-4" />
						{{ formatMessage(messages.startHosting) }}
					</Button>
				</div>
			</div>

			<!-- View 2: Active hosting -->
			<div v-else class="flex flex-col gap-4">
				<!-- Status Banner -->
				<div class="flex items-center justify-between rounded-xl bg-green-500/10 p-3 text-xs">
					<div class="flex items-center gap-2 text-green-400 font-semibold">
						<span class="size-2 rounded-full bg-green-400 animate-pulse" />
						<span>{{ formatMessage(messages.activeHosting, { port: serverPort }) }}</span>
					</div>
					<Button size="sm" type="outlined" :disabled="loading" @click="handleStopHost">
						<StopCircleIcon class="size-3.5" />
						{{ formatMessage(messages.stopHosting) }}
					</Button>
				</div>

				<!-- Invite Code Display -->
				<div
					class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
				>
					<div class="flex items-center justify-between">
						<span class="text-xs font-semibold text-contrast">
							{{ formatMessage(messages.inviteCodeLabel) }}
						</span>
						<span class="text-[11px] text-secondary">
							{{ formatMessage(messages.securityNote) }}
						</span>
					</div>

					<div class="flex items-center gap-2">
						<input
							type="text"
							readonly
							:value="inviteCode"
							class="w-full select-all rounded-lg border border-solid border-surface-5 bg-surface-1 px-3 py-2 font-mono text-xs text-contrast outline-none focus:border-brand"
						/>
						<Button size="md" type="colored" color="brand" @click="copyInviteCode">
							<CheckIcon v-if="copied" class="size-4" />
							<CopyIcon v-else class="size-4" />
							{{ copied ? formatMessage(messages.copied) : formatMessage(messages.copyCode) }}
						</Button>
					</div>

					<div class="mt-1 flex items-center justify-between text-xs">
						<Button size="sm" type="transparent" :disabled="loading" @click="handleRegenerateCode">
							<RefreshCwIcon class="size-3.5" :class="{ 'animate-spin': loading }" />
							{{ formatMessage(messages.regenerate) }}
						</Button>
						<Button
							size="sm"
							type="transparent"
							class="text-red-400 hover:text-red-300"
							:disabled="loading"
							@click="handleRevokeCode"
						>
							<XIcon class="size-3.5" />
							{{ formatMessage(messages.revoke) }}
						</Button>
					</div>
				</div>

				<!-- Connected Peers -->
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between text-xs">
						<span class="font-semibold text-contrast">
							{{ formatMessage(messages.connectedFriends) }} ({{ connectedPeers.length }}/{{
								maxPeers
							}})
						</span>
					</div>

					<div
						v-if="connectedPeers.length === 0"
						class="rounded-xl border border-dashed border-surface-5 p-4 text-center text-xs text-secondary"
					>
						{{ formatMessage(messages.noConnectedFriends) }}
					</div>

					<div v-else class="flex flex-col gap-1.5 max-h-40 overflow-y-auto">
						<div
							v-for="peer in connectedPeers"
							:key="peer.peer_id"
							class="flex items-center justify-between rounded-lg bg-surface-2 px-3 py-2 text-xs"
						>
							<div class="flex items-center gap-2">
								<UsersIcon class="size-3.5 text-secondary" />
								<span class="font-mono text-contrast"> {{ peer.peer_id.slice(0, 14) }}... </span>
							</div>
							<span
								:class="[
									'rounded px-2 py-0.5 text-[11px] font-medium',
									peer.is_direct
										? 'bg-green-500/20 text-green-400'
										: 'bg-blue-500/20 text-blue-400',
								]"
							>
								{{
									peer.is_direct ? formatMessage(messages.direct) : formatMessage(messages.relayed)
								}}
							</span>
						</div>
					</div>
				</div>

				<!-- Footer -->
				<div class="flex justify-end pt-2 border-t border-solid border-surface-5">
					<Button type="outlined" @click="hide">
						{{ formatMessage(commonMessages.closeButton) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckIcon,
	CopyIcon,
	RefreshCwIcon,
	ShareIcon,
	ShieldCheckIcon,
	StopCircleIcon,
	UsersIcon,
	XIcon,
} from '@modrinth/assets'
import { Button, commonMessages, defineMessages, Input, NewModal, useVIntl } from '@modrinth/ui'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { onBeforeUnmount, ref } from 'vue'

import {
	type ConnectedPeer,
	getP2PHostInfo,
	onP2PHostEvent,
	regenerateInviteCode,
	respondJoinRequest,
	revokeInviteCode,
	startP2PHost,
	stopP2PHost,
} from '@/helpers/p2p'

const { formatMessage } = useVIntl()

const modalRef = ref<InstanceType<typeof NewModal>>()
const isHosting = ref(false)
const inviteCode = ref('')
const serverPort = ref(25565)
const maxPeers = ref(8)
const requireApproval = ref(true)
const rememberSession = ref(false)
const connectedPeers = ref<ConnectedPeer[]>([])
const pendingRequests = ref<{ peer_id: string }[]>([])
const loading = ref(false)
const copied = ref(false)
const error = ref<string | null>(null)

let unlistenHostEvents: UnlistenFn | null = null

const messages = defineMessages({
	title: {
		id: 'p2p.host.title',
		defaultMessage: 'Share Server via P2P',
	},
	subtitle: {
		id: 'p2p.host.subtitle',
		defaultMessage:
			'Tunnel your local Minecraft/Gradle server over peer-to-peer QUIC with automatic NAT hole punching and relay fallback.',
	},
	port: {
		id: 'p2p.host.port',
		defaultMessage: 'Local Server Port',
	},
	maxPeers: {
		id: 'p2p.host.max-peers',
		defaultMessage: 'Max Friends',
	},
	requireApprovalLabel: {
		id: 'p2p.host.require-approval-label',
		defaultMessage: 'Require Friend Approval',
	},
	requireApprovalDesc: {
		id: 'p2p.host.require-approval-desc',
		defaultMessage: 'Prompt to allow or deny each incoming friend connection',
	},
	startHosting: {
		id: 'p2p.host.start-hosting',
		defaultMessage: 'Start P2P Host',
	},
	activeHosting: {
		id: 'p2p.host.active-hosting',
		defaultMessage: 'Hosting active (Port {port})',
	},
	stopHosting: {
		id: 'p2p.host.stop-hosting',
		defaultMessage: 'Stop Host',
	},
	inviteCodeLabel: {
		id: 'p2p.host.invite-code-label',
		defaultMessage: 'Connection Invite Code',
	},
	securityNote: {
		id: 'p2p.host.security-note',
		defaultMessage: 'Memory only · Never logged',
	},
	copyCode: {
		id: 'p2p.host.copy-code',
		defaultMessage: 'Copy Code',
	},
	copied: {
		id: 'p2p.host.copied',
		defaultMessage: 'Copied!',
	},
	regenerate: {
		id: 'p2p.host.regenerate',
		defaultMessage: 'Regenerate Code',
	},
	revoke: {
		id: 'p2p.host.revoke',
		defaultMessage: 'Revoke Code & Disconnect All',
	},
	connectedFriends: {
		id: 'p2p.host.connected-friends',
		defaultMessage: 'Connected Friends',
	},
	noConnectedFriends: {
		id: 'p2p.host.no-connected-friends',
		defaultMessage: 'No friends connected yet. Send them your invite code!',
	},
	direct: {
		id: 'p2p.host.direct',
		defaultMessage: 'Direct (P2P)',
	},
	relayed: {
		id: 'p2p.host.relayed',
		defaultMessage: 'Relayed',
	},
	joinRequestPrompt: {
		id: 'p2p.host.join-request-prompt',
		defaultMessage: 'Friend Join Request',
	},
	allow: {
		id: 'p2p.host.allow',
		defaultMessage: 'Allow',
	},
	deny: {
		id: 'p2p.host.deny',
		defaultMessage: 'Deny',
	},
	rememberApproval: {
		id: 'p2p.host.remember-approval',
		defaultMessage: 'Remember approval for this session',
	},
})

async function refreshHostInfo() {
	try {
		const info = await getP2PHostInfo()
		isHosting.value = info.is_hosting
		inviteCode.value = info.invite_code ?? ''
		connectedPeers.value = info.connected_peers
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	}
}

async function handleStartHost() {
	loading.value = true
	error.value = null
	try {
		const code = await startP2PHost(serverPort.value, maxPeers.value, requireApproval.value)
		inviteCode.value = code
		isHosting.value = true
		await refreshHostInfo()
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		loading.value = false
	}
}

async function handleStopHost() {
	loading.value = true
	try {
		await stopP2PHost()
		isHosting.value = false
		inviteCode.value = ''
		connectedPeers.value = []
		pendingRequests.value = []
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		loading.value = false
	}
}

async function handleRegenerateCode() {
	loading.value = true
	try {
		const code = await regenerateInviteCode()
		inviteCode.value = code
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		loading.value = false
	}
}

async function handleRevokeCode() {
	loading.value = true
	try {
		await revokeInviteCode()
		inviteCode.value = ''
		connectedPeers.value = []
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		loading.value = false
	}
}

async function handleRespondRequest(peerId: string, allow: boolean) {
	try {
		await respondJoinRequest(peerId, allow, rememberSession.value)
		pendingRequests.value = pendingRequests.value.filter((r) => r.peer_id !== peerId)
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	}
}

async function copyInviteCode() {
	if (!inviteCode.value) return
	try {
		await navigator.clipboard.writeText(inviteCode.value)
		copied.value = true
		setTimeout(() => {
			copied.value = false
		}, 2000)
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	}
}

async function setupListener() {
	if (unlistenHostEvents) return
	unlistenHostEvents = await onP2PHostEvent((evt) => {
		if (evt.type === 'JoinRequest') {
			if (!pendingRequests.value.some((r) => r.peer_id === evt.data.peer_id)) {
				pendingRequests.value.push({ peer_id: evt.data.peer_id })
			}
		} else if (evt.type === 'PeerConnected') {
			connectedPeers.value = connectedPeers.value.filter((p) => p.peer_id !== evt.data.peer_id)
			connectedPeers.value.push({
				peer_id: evt.data.peer_id,
				is_direct: evt.data.is_direct,
			})
		} else if (evt.type === 'PeerDisconnected') {
			connectedPeers.value = connectedPeers.value.filter((p) => p.peer_id !== evt.data.peer_id)
		} else if (evt.type === 'InviteRevoked') {
			inviteCode.value = ''
			connectedPeers.value = []
		} else if (evt.type === 'InviteRegenerated') {
			inviteCode.value = evt.data.code
		}
	})
}

async function show(initialPort?: number) {
	if (initialPort) {
		serverPort.value = initialPort
	}
	error.value = null
	await setupListener()
	await refreshHostInfo()
	modalRef.value?.show()
}

function hide() {
	modalRef.value?.hide()
}

onBeforeUnmount(() => {
	if (unlistenHostEvents) {
		unlistenHostEvents()
		unlistenHostEvents = null
	}
})

defineExpose({ show, hide })
</script>
