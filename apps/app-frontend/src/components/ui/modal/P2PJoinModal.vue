<template>
	<NewModal ref="modalRef" :header="formatMessage(messages.title)" max-width="540px" width="540px">
		<div class="flex w-full flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.subtitle) }}
			</p>

			<!-- Error banner -->
			<div v-if="error" class="rounded-xl bg-red-500/10 p-3 text-xs text-red-400">
				{{ error }}
			</div>

			<!-- State 1: Input invite code and connect -->
			<div v-if="!isConnected" class="flex flex-col gap-3">
				<label class="flex flex-col gap-1.5 text-xs text-secondary">
					<span class="font-semibold text-contrast">
						{{ formatMessage(messages.inviteCodePrompt) }}
					</span>
					<textarea
						v-model="inviteCodeInput"
						rows="3"
						:placeholder="formatMessage(messages.codePlaceholder)"
						class="w-full resize-none rounded-xl border border-solid border-surface-5 bg-surface-2 p-3 font-mono text-xs text-contrast outline-none focus:border-brand"
					/>
				</label>

				<div class="flex justify-end gap-2 pt-2">
					<Button type="transparent" @click="hide">
						{{ formatMessage(commonMessages.cancelButton) }}
					</Button>
					<Button
						type="colored"
						color="brand"
						:disabled="!inviteCodeInput.trim() || connecting"
						@click="handleJoin"
					>
						<SpinnerIcon v-if="connecting" class="size-4 animate-spin" />
						<PlayIcon v-else class="size-4" />
						{{
							connecting
								? formatMessage(messages.connecting)
								: formatMessage(messages.connectButton)
						}}
					</Button>
				</div>
			</div>

			<!-- State 2: Active Connection & Port Tunnel -->
			<div v-else class="flex flex-col gap-4">
				<!-- Status Badge -->
				<div
					:class="[
						'flex items-center gap-2 rounded-xl p-3 text-xs font-semibold',
						statusType === 'Direct'
							? 'bg-green-500/10 text-green-400'
							: statusType === 'Relayed'
								? 'bg-blue-500/10 text-blue-400'
								: statusType === 'Failed'
									? 'bg-red-500/10 text-red-400'
									: 'bg-surface-3 text-secondary',
					]"
				>
					<CheckIcon v-if="statusType === 'Direct'" class="size-4 shrink-0" />
					<GlobeIcon v-else-if="statusType === 'Relayed'" class="size-4 shrink-0" />
					<SpinnerIcon
						v-else-if="statusType === 'Connecting'"
						class="size-4 animate-spin shrink-0"
					/>
					<XIcon v-else class="size-4 shrink-0" />

					<span>{{ statusDescription }}</span>
				</div>

				<!-- Local Port Forwarding Card -->
				<div
					class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
				>
					<div class="flex items-center justify-between">
						<span class="text-xs font-semibold text-contrast">
							{{ formatMessage(messages.localAddressLabel) }}
						</span>
						<span class="rounded bg-brand/10 px-2 py-0.5 text-[11px] font-medium text-brand">
							{{ formatMessage(messages.tunnelReady) }}
						</span>
					</div>

					<div class="flex items-center gap-2">
						<input
							type="text"
							readonly
							:value="`127.0.0.1:${localPort}`"
							class="w-full select-all rounded-lg border border-solid border-surface-5 bg-surface-1 px-3 py-2 font-mono text-xs font-bold text-contrast outline-none"
						/>
						<Button size="md" type="colored" color="brand" @click="copyAddress">
							<CheckIcon v-if="copied" class="size-4" />
							<CopyIcon v-else class="size-4" />
							{{ copied ? formatMessage(messages.copied) : formatMessage(messages.copyAddress) }}
						</Button>
					</div>

					<p class="m-0 text-xs text-secondary">
						{{ formatMessage(messages.connectionHint, { port: localPort }) }}
					</p>
				</div>

				<!-- Actions -->
				<div class="flex items-center justify-between border-t border-solid border-surface-5 pt-3">
					<Button type="colored" color="red" size="md" @click="handleDisconnect">
						<StopCircleIcon class="size-4" />
						{{ formatMessage(messages.disconnectButton) }}
					</Button>

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
	GlobeIcon,
	PlayIcon,
	SpinnerIcon,
	StopCircleIcon,
	XIcon,
} from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, ref } from 'vue'

import {
	disconnectP2PJoin,
	getP2PJoinInfo,
	joinP2P,
	onP2PJoinStatus,
	type TunnelStatus,
} from '@/helpers/p2p'

const { formatMessage } = useVIntl()

const modalRef = ref<InstanceType<typeof NewModal>>()
const inviteCodeInput = ref('')
const isConnected = ref(false)
const localPort = ref<number | null>(null)
const status = ref<TunnelStatus | null>(null)
const connecting = ref(false)
const copied = ref(false)
const error = ref<string | null>(null)

let unlistenStatus: UnlistenFn | null = null

const messages = defineMessages({
	title: {
		id: 'p2p.join.title',
		defaultMessage: 'Join Server via Invite Code',
	},
	subtitle: {
		id: 'p2p.join.subtitle',
		defaultMessage:
			'Paste your friend’s invite code to establish an encrypted peer-to-peer game tunnel directly to their server.',
	},
	inviteCodePrompt: {
		id: 'p2p.join.invite-code-prompt',
		defaultMessage: 'P2P Invite Code',
	},
	codePlaceholder: {
		id: 'p2p.join.code-placeholder',
		defaultMessage: 'Paste gmc1_... invite code here',
	},
	connectButton: {
		id: 'p2p.join.connect-button',
		defaultMessage: 'Connect to Server',
	},
	connecting: {
		id: 'p2p.join.connecting',
		defaultMessage: 'Connecting...',
	},
	localAddressLabel: {
		id: 'p2p.join.local-address-label',
		defaultMessage: 'Forwarded Minecraft Address',
	},
	tunnelReady: {
		id: 'p2p.join.tunnel-ready',
		defaultMessage: 'P2P Tunnel Ready',
	},
	copyAddress: {
		id: 'p2p.join.copy-address',
		defaultMessage: 'Copy Address',
	},
	copied: {
		id: 'p2p.join.copied',
		defaultMessage: 'Copied!',
	},
	connectionHint: {
		id: 'p2p.join.connection-hint',
		defaultMessage: 'Connect inside Minecraft via Direct Connect or Add Server to 127.0.0.1:{port}',
	},
	disconnectButton: {
		id: 'p2p.join.disconnect-button',
		defaultMessage: 'Disconnect Tunnel',
	},
	directStatus: {
		id: 'p2p.join.direct-status',
		defaultMessage: 'Connected directly via NAT hole punching ({addr})',
	},
	relayedStatus: {
		id: 'p2p.join.relayed-status',
		defaultMessage: 'Connected via encrypted Iroh relay fallback',
	},
	failedStatus: {
		id: 'p2p.join.failed-status',
		defaultMessage: 'Connection failed: {reason}',
	},
	disconnectedStatus: {
		id: 'p2p.join.disconnected-status',
		defaultMessage: 'Tunnel disconnected',
	},
	connectingStatus: {
		id: 'p2p.join.connecting-status',
		defaultMessage: 'Establishing QUIC connection with host...',
	},
})

const statusType = computed(() => {
	if (!status.value) return 'Connecting'
	return status.value.type
})

const statusDescription = computed(() => {
	if (!status.value) return formatMessage(messages.connectingStatus)
	switch (status.value.type) {
		case 'Direct':
			return formatMessage(messages.directStatus, { addr: status.value.data.remote_addr })
		case 'Relayed':
			return formatMessage(messages.relayedStatus)
		case 'Failed':
			return formatMessage(messages.failedStatus, { reason: status.value.data.reason })
		case 'Disconnected':
			return formatMessage(messages.disconnectedStatus)
		case 'Connecting':
		default:
			return formatMessage(messages.connectingStatus)
	}
})

async function checkJoinInfo() {
	try {
		const info = await getP2PJoinInfo()
		isConnected.value = info.is_connected
		localPort.value = info.local_port
		status.value = info.status
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	}
}

async function handleJoin() {
	const code = inviteCodeInput.value.trim()
	if (!code) return

	connecting.value = true
	error.value = null

	try {
		await setupStatusListener()
		const port = await joinP2P(code)
		localPort.value = port
		isConnected.value = true
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
		isConnected.value = false
	} finally {
		connecting.value = false
	}
}

async function handleDisconnect() {
	try {
		await disconnectP2PJoin()
		isConnected.value = false
		localPort.value = null
		status.value = null
		inviteCodeInput.value = ''
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	}
}

async function copyAddress() {
	if (!localPort.value) return
	try {
		await navigator.clipboard.writeText(`127.0.0.1:${localPort.value}`)
		copied.value = true
		setTimeout(() => {
			copied.value = false
		}, 2000)
	} catch (e: unknown) {
		error.value = e instanceof Error ? e.message : String(e)
	}
}

async function setupStatusListener() {
	if (unlistenStatus) return
	unlistenStatus = await onP2PJoinStatus((newStatus) => {
		status.value = newStatus
		if (newStatus.type === 'Disconnected' || newStatus.type === 'Failed') {
			if (newStatus.type === 'Failed') {
				error.value = newStatus.data.reason
			}
		}
	})
}

async function show() {
	error.value = null
	await setupStatusListener()
	await checkJoinInfo()
	modalRef.value?.show()
}

function hide() {
	modalRef.value?.hide()
}

onBeforeUnmount(() => {
	if (unlistenStatus) {
		unlistenStatus()
		unlistenStatus = null
	}
})

defineExpose({ show, hide })
</script>
