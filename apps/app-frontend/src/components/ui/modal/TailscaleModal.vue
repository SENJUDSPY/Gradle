<template>
	<NewModal ref="modalRef" :header="formatMessage(messages.title)" max-width="540px" width="540px">
		<div class="flex w-full flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.subtitle) }}
			</p>

			<!-- State 1: Tailscale detected and active -->
			<div v-if="tailscaleIp" class="flex flex-col gap-3">
				<div class="flex items-center justify-between rounded-xl bg-green-500/10 p-3 text-xs">
					<div class="flex items-center gap-2 font-semibold text-green-400">
						<span class="size-2 rounded-full bg-green-400 animate-pulse" />
						<span>{{ formatMessage(messages.tailscaleActive) }}</span>
					</div>
					<span class="font-mono text-secondary">{{ tailscaleIp }}</span>
				</div>

				<div
					class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
				>
					<span class="text-xs font-semibold text-contrast">
						{{ formatMessage(messages.tailscaleAddressLabel) }}
					</span>

					<div class="flex items-center gap-2">
						<input
							type="text"
							readonly
							:value="`${tailscaleIp}:${serverPort}`"
							class="w-full select-all rounded-lg border border-solid border-surface-5 bg-surface-1 px-3 py-2 font-mono text-xs font-bold text-contrast outline-none"
						/>
						<Button size="md" type="colored" color="brand" @click="copyAddress">
							<CheckIcon v-if="copied" class="size-4" />
							<CopyIcon v-else class="size-4" />
							{{ copied ? formatMessage(messages.copied) : formatMessage(messages.copyAddress) }}
						</Button>
					</div>

					<p class="m-0 text-xs text-secondary">
						{{ formatMessage(messages.tailscaleConnectHint) }}
					</p>
				</div>
			</div>

			<!-- State 2: Tailscale not running or not logged in -->
			<div
				v-else
				class="flex flex-col gap-3 rounded-xl border border-solid border-surface-5 bg-surface-2 p-4 text-xs text-secondary"
			>
				<div class="flex items-center gap-2 font-semibold text-contrast">
					<GlobeIcon class="size-4 text-brand" />
					<span>{{ formatMessage(messages.setupGuideTitle) }}</span>
				</div>

				<ol class="m-0 flex flex-col gap-2 pl-4 text-secondary">
					<li>{{ formatMessage(messages.step1) }}</li>
					<li>{{ formatMessage(messages.step2) }}</li>
					<li>{{ formatMessage(messages.step3, { port: serverPort }) }}</li>
				</ol>

				<div class="mt-2 rounded-lg bg-surface-3 p-2.5 font-mono text-[11px] text-contrast">
					tailscale up
				</div>
			</div>

			<div class="flex justify-end pt-2 border-t border-solid border-surface-5">
				<Button type="outlined" @click="hide">
					{{ formatMessage(commonMessages.closeButton) }}
				</Button>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, CopyIcon, GlobeIcon } from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import { getTailscaleIp } from '@/helpers/horizon'

const { formatMessage } = useVIntl()

const modalRef = ref<InstanceType<typeof NewModal>>()
const tailscaleIp = ref<string | null>(null)
const serverPort = ref(25565)
const copied = ref(false)

const messages = defineMessages({
	title: {
		id: 'tailscale.modal.title',
		defaultMessage: 'Tailscale Server Sharing',
	},
	subtitle: {
		id: 'tailscale.modal.subtitle',
		defaultMessage:
			'Invite friends through your Tailscale private mesh network (tailnet) with zero configuration or port forwarding.',
	},
	tailscaleActive: {
		id: 'tailscale.modal.active',
		defaultMessage: 'Tailscale VPN Connected',
	},
	tailscaleAddressLabel: {
		id: 'tailscale.modal.address-label',
		defaultMessage: 'Your Tailscale Minecraft Server IP',
	},
	tailscaleConnectHint: {
		id: 'tailscale.modal.connect-hint',
		defaultMessage:
			'Any friend connected to your Tailscale network can enter this IP directly into their Minecraft multiplayer menu.',
	},
	setupGuideTitle: {
		id: 'tailscale.modal.setup-guide-title',
		defaultMessage: 'How to use Tailscale with Gradle',
	},
	step1: {
		id: 'tailscale.modal.step-1',
		defaultMessage:
			'Install Tailscale on your PC and invite friends to your tailnet (tailscale.com).',
	},
	step2: {
		id: 'tailscale.modal.step-2',
		defaultMessage: 'Run `tailscale up` to connect to your private mesh.',
	},
	step3: {
		id: 'tailscale.modal.step-3',
		defaultMessage: 'Friends on your tailnet can connect to your 100.x.y.z IP on port {port}.',
	},
	copyAddress: {
		id: 'tailscale.modal.copy-address',
		defaultMessage: 'Copy Tailscale Address',
	},
	copied: {
		id: 'tailscale.modal.copied',
		defaultMessage: 'Copied!',
	},
})

async function copyAddress() {
	if (!tailscaleIp.value) return
	try {
		await navigator.clipboard.writeText(`${tailscaleIp.value}:${serverPort.value}`)
		copied.value = true
		setTimeout(() => {
			copied.value = false
		}, 2000)
	} catch {
		// ignore
	}
}

async function show(port?: number) {
	if (port) {
		serverPort.value = port
	}
	try {
		tailscaleIp.value = await getTailscaleIp()
	} catch {
		tailscaleIp.value = null
	}
	modalRef.value?.show()
}

function hide() {
	modalRef.value?.hide()
}

defineExpose({ show, hide })
</script>
