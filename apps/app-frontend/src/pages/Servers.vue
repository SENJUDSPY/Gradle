<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	BoxIcon,
	GlobeIcon,
	PlayIcon,
	PlusIcon,
	ServerIcon,
	ServerStackIcon,
	ShareIcon,
	SpinnerIcon,
	StopCircleIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	injectModrinthClient,
	Input,
	NewModal,
	ServersManagePageIndex,
	useVIntl,
} from '@modrinth/ui'
import { useQuery } from '@tanstack/vue-query'
import { computed, onMounted, ref } from 'vue'

import P2PHostModal from '@/components/ui/modal/P2PHostModal.vue'
import P2PJoinModal from '@/components/ui/modal/P2PJoinModal.vue'
import TailscaleModal from '@/components/ui/modal/TailscaleModal.vue'
import {
	backupHorizonServer,
	createHorizonServer,
	deleteHorizonServer,
	type HorizonServerInfo,
	listHorizonServers,
	startHorizonServer,
	stopHorizonServer,
} from '@/helpers/horizon'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

import { config } from '../config'

const stripePublishableKey = (config.stripePublishableKey as string) || ''
const client = injectModrinthClient()
const { formatMessage } = useVIntl()

useRootBreadcrumb({
	slot: 'root',
	id: 'servers',
	label: 'Hosting',
	to: '/hosting/manage/',
	visual: { type: 'icon', component: ServerStackIcon },
})

const { data: products } = useQuery({
	queryKey: ['billing', 'products'],
	queryFn: () => client.labrinth.billing_internal.getProducts(),
})

const resolvedProducts = computed<Labrinth.Billing.Internal.Product[]>(() => products.value ?? [])

const activeTab = ref<'horizon' | 'cloud'>('horizon')
const servers = ref<HorizonServerInfo[]>([])
const loadingServers = ref(false)
const serverActionLoading = ref<string | null>(null)
const serverMessage = ref<string | null>(null)
const serverError = ref<string | null>(null)

// Modals
const p2pHostModal = ref<InstanceType<typeof P2PHostModal>>()
const p2pJoinModal = ref<InstanceType<typeof P2PJoinModal>>()
const tailscaleModal = ref<InstanceType<typeof TailscaleModal>>()
const createServerModal = ref<InstanceType<typeof NewModal>>()

// Create form
const newServerName = ref('Horizon Server')
const newServerPort = ref(25565)
const creatingServer = ref(false)

const messages = defineMessages({
	tabHorizon: {
		id: 'servers.page.tab.horizon',
		defaultMessage: 'Horizon Dedicated Servers',
	},
	tabCloud: {
		id: 'servers.page.tab.cloud',
		defaultMessage: 'Modrinth Cloud Hosting',
	},
	horizonTitle: {
		id: 'servers.page.horizon-title',
		defaultMessage: 'Native Rust Dedicated Servers',
	},
	horizonSubtitle: {
		id: 'servers.page.horizon-subtitle',
		defaultMessage:
			'Run standalone Minecraft servers natively in Rust with zero JVM overhead. Invite friends via P2P invite codes (no port forwarding required) or Tailscale.',
	},
	joinWithCode: {
		id: 'servers.page.join-with-code',
		defaultMessage: 'Join via Invite Code',
	},
	createServer: {
		id: 'servers.page.create-server',
		defaultMessage: 'Create Horizon Server',
	},
	createModalTitle: {
		id: 'servers.page.create-modal-title',
		defaultMessage: 'Create Native Horizon Server',
	},
	serverNameLabel: {
		id: 'servers.page.server-name-label',
		defaultMessage: 'Server Name',
	},
	serverPortLabel: {
		id: 'servers.page.server-port-label',
		defaultMessage: 'Server Port',
	},
	noServersTitle: {
		id: 'servers.page.no-servers-title',
		defaultMessage: 'No Horizon servers yet',
	},
	noServersDesc: {
		id: 'servers.page.no-servers-desc',
		defaultMessage:
			'Create a native Minecraft 26.3 server that starts instantly, uses under 150MB of RAM, and can be shared over peer-to-peer invite tunnels.',
	},
	startServer: {
		id: 'servers.page.start-server',
		defaultMessage: 'Start Server',
	},
	stopServer: {
		id: 'servers.page.stop-server',
		defaultMessage: 'Stop Server',
	},
	shareP2P: {
		id: 'servers.page.share-p2p',
		defaultMessage: 'Share via P2P',
	},
	tailscale: {
		id: 'servers.page.tailscale',
		defaultMessage: 'Tailscale',
	},
	backupWorld: {
		id: 'servers.page.backup-world',
		defaultMessage: 'Backup',
	},
	deleteServer: {
		id: 'servers.page.delete-server',
		defaultMessage: 'Delete',
	},
	running: {
		id: 'servers.page.running',
		defaultMessage: 'Running (Port {port})',
	},
	stopped: {
		id: 'servers.page.stopped',
		defaultMessage: 'Stopped',
	},
	backupsCount: {
		id: 'servers.page.backups-count',
		defaultMessage: '{count} backups',
	},
	backupSuccess: {
		id: 'servers.page.backup-success',
		defaultMessage: 'World backup saved successfully!',
	},
})

async function fetchServers() {
	loadingServers.value = true
	serverError.value = null
	try {
		servers.value = await listHorizonServers()
	} catch (e: unknown) {
		serverError.value = e instanceof Error ? e.message : String(e)
	} finally {
		loadingServers.value = false
	}
}

function openCreateModal() {
	newServerName.value = `Horizon Server ${servers.value.length + 1}`
	newServerPort.value = 25565 + servers.value.length
	serverError.value = null
	createServerModal.value?.show()
}

async function handleCreateServer() {
	creatingServer.value = true
	serverError.value = null
	try {
		await createHorizonServer(newServerName.value.trim(), null, newServerPort.value)
		createServerModal.value?.hide()
		await fetchServers()
	} catch (e: unknown) {
		serverError.value = e instanceof Error ? e.message : String(e)
	} finally {
		creatingServer.value = false
	}
}

async function handleStartServer(name: string) {
	serverActionLoading.value = name
	serverError.value = null
	serverMessage.value = null
	try {
		await startHorizonServer(name)
		await fetchServers()
	} catch (e: unknown) {
		serverError.value = e instanceof Error ? e.message : String(e)
	} finally {
		serverActionLoading.value = null
	}
}

async function handleStopServer(name: string) {
	serverActionLoading.value = name
	serverError.value = null
	serverMessage.value = null
	try {
		await stopHorizonServer(name)
		await fetchServers()
	} catch (e: unknown) {
		serverError.value = e instanceof Error ? e.message : String(e)
	} finally {
		serverActionLoading.value = null
	}
}

async function handleBackupServer(name: string) {
	serverActionLoading.value = name
	serverError.value = null
	serverMessage.value = null
	try {
		await backupHorizonServer(name)
		serverMessage.value = formatMessage(messages.backupSuccess)
		await fetchServers()
		setTimeout(() => {
			serverMessage.value = null
		}, 3000)
	} catch (e: unknown) {
		serverError.value = e instanceof Error ? e.message : String(e)
	} finally {
		serverActionLoading.value = null
	}
}

async function handleDeleteServer(name: string) {
	serverActionLoading.value = name
	serverError.value = null
	serverMessage.value = null
	try {
		await deleteHorizonServer(name)
		await fetchServers()
	} catch (e: unknown) {
		serverError.value = e instanceof Error ? e.message : String(e)
	} finally {
		serverActionLoading.value = null
	}
}

function openP2PHost(port: number) {
	p2pHostModal.value?.show(port)
}

function openP2PJoin() {
	p2pJoinModal.value?.show()
}

function openTailscale(port: number) {
	tailscaleModal.value?.show(port)
}

onMounted(() => {
	fetchServers()
})
</script>

<template>
	<div class="flex flex-col w-full h-full p-6 max-w-7xl mx-auto gap-6 overflow-y-auto">
		<!-- Top View Switcher -->
		<div class="flex items-center justify-between border-b border-solid border-surface-5 pb-4">
			<div class="flex items-center rounded-xl bg-surface-2 p-1 text-xs font-semibold">
				<button
					type="button"
					:class="[
						'flex items-center gap-1.5 rounded-lg px-3 py-1.5 transition-all',
						activeTab === 'horizon'
							? 'bg-brand text-white shadow-sm'
							: 'text-secondary hover:text-contrast',
					]"
					@click="activeTab = 'horizon'"
				>
					<ServerIcon class="size-4" />
					{{ formatMessage(messages.tabHorizon) }}
				</button>
				<button
					type="button"
					:class="[
						'flex items-center gap-1.5 rounded-lg px-3 py-1.5 transition-all',
						activeTab === 'cloud'
							? 'bg-brand text-white shadow-sm'
							: 'text-secondary hover:text-contrast',
					]"
					@click="activeTab = 'cloud'"
				>
					<ServerStackIcon class="size-4" />
					{{ formatMessage(messages.tabCloud) }}
				</button>
			</div>

			<!-- Quick Join Code Button -->
			<div v-if="activeTab === 'horizon'" class="flex items-center gap-2">
				<Button size="md" type="outlined" @click="openP2PJoin">
					<GlobeIcon class="size-4" />
					{{ formatMessage(messages.joinWithCode) }}
				</Button>

				<Button size="md" type="colored" color="brand" @click="openCreateModal">
					<PlusIcon class="size-4" />
					{{ formatMessage(messages.createServer) }}
				</Button>
			</div>
		</div>

		<!-- View 1: Horizon Dedicated Servers -->
		<div v-if="activeTab === 'horizon'" class="flex flex-col gap-6">
			<!-- Banner info -->
			<div class="flex flex-col gap-1">
				<h2 class="m-0 text-xl font-bold text-contrast">
					{{ formatMessage(messages.horizonTitle) }}
				</h2>
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.horizonSubtitle) }}
				</p>
			</div>

			<!-- Success / Error alerts -->
			<div
				v-if="serverMessage"
				class="rounded-xl bg-green-500/10 p-3 text-xs font-medium text-green-400"
			>
				{{ serverMessage }}
			</div>

			<div v-if="serverError" class="rounded-xl bg-red-500/10 p-3 text-xs text-red-400">
				{{ serverError }}
			</div>

			<!-- Empty State -->
			<div
				v-if="servers.length === 0 && !loadingServers"
				class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-surface-5 bg-surface-2 p-12 text-center"
			>
				<div
					class="flex size-14 items-center justify-center rounded-2xl bg-brand/10 text-brand mb-4"
				>
					<ServerIcon class="size-8" />
				</div>
				<h3 class="m-0 text-base font-bold text-contrast">
					{{ formatMessage(messages.noServersTitle) }}
				</h3>
				<p class="m-0 mt-2 max-w-md text-sm text-secondary">
					{{ formatMessage(messages.noServersDesc) }}
				</p>
				<Button type="colored" color="brand" size="lg" class="mt-6" @click="openCreateModal">
					<PlusIcon class="size-4" />
					{{ formatMessage(messages.createServer) }}
				</Button>
			</div>

			<!-- Servers List -->
			<div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
				<div
					v-for="srv in servers"
					:key="srv.name"
					class="flex flex-col justify-between rounded-2xl border border-solid border-surface-5 bg-surface-2 p-5 transition-all hover:border-surface-4"
				>
					<div class="flex flex-col gap-3">
						<div class="flex items-start justify-between">
							<div class="flex flex-col gap-0.5">
								<span class="text-base font-bold text-contrast">{{ srv.name }}</span>
								<span class="text-xs text-secondary font-mono">{{ srv.config.motd }}</span>
							</div>

							<!-- Status badge -->
							<span
								:class="[
									'flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-semibold',
									srv.is_running ? 'bg-green-500/10 text-green-400' : 'bg-surface-4 text-secondary',
								]"
							>
								<span
									:class="[
										'size-2 rounded-full',
										srv.is_running ? 'bg-green-400 animate-pulse' : 'bg-surface-5',
									]"
								/>
								{{
									srv.is_running
										? formatMessage(messages.running, { port: srv.config.port })
										: formatMessage(messages.stopped)
								}}
							</span>
						</div>

						<!-- Metadata badges -->
						<div class="flex items-center gap-2 text-xs text-secondary">
							<span class="rounded bg-surface-3 px-2 py-0.5 font-mono">
								v{{ srv.config.tag }}
							</span>
							<span class="rounded bg-surface-3 px-2 py-0.5 font-mono">
								{{ formatMessage(messages.backupsCount, { count: srv.backup_count }) }}
							</span>
						</div>
					</div>

					<!-- Server Actions -->
					<div
						class="mt-5 flex items-center justify-between border-t border-solid border-surface-4 pt-4"
					>
						<div class="flex items-center gap-2">
							<Button
								v-if="srv.is_running"
								size="sm"
								type="colored"
								color="red"
								:disabled="serverActionLoading === srv.name"
								@click="handleStopServer(srv.name)"
							>
								<StopCircleIcon class="size-3.5" />
								{{ formatMessage(messages.stopServer) }}
							</Button>
							<Button
								v-else
								size="sm"
								type="colored"
								color="brand"
								:disabled="serverActionLoading === srv.name"
								@click="handleStartServer(srv.name)"
							>
								<PlayIcon class="size-3.5" />
								{{ formatMessage(messages.startServer) }}
							</Button>

							<Button
								v-if="srv.is_running"
								size="sm"
								type="colored"
								color="brand"
								@click="openP2PHost(srv.config.port)"
							>
								<ShareIcon class="size-3.5" />
								{{ formatMessage(messages.shareP2P) }}
							</Button>

							<Button
								v-if="srv.is_running"
								size="sm"
								type="outlined"
								@click="openTailscale(srv.config.port)"
							>
								<GlobeIcon class="size-3.5" />
								{{ formatMessage(messages.tailscale) }}
							</Button>
						</div>

						<div class="flex items-center gap-1.5">
							<Button
								size="sm"
								type="outlined"
								:disabled="serverActionLoading === srv.name"
								@click="handleBackupServer(srv.name)"
							>
								<BoxIcon class="size-3.5" />
								{{ formatMessage(messages.backupWorld) }}
							</Button>
							<Button
								v-if="!srv.is_running"
								size="sm"
								type="transparent"
								class="text-red-400 hover:text-red-300"
								:disabled="serverActionLoading === srv.name"
								@click="handleDeleteServer(srv.name)"
							>
								<TrashIcon class="size-3.5" />
							</Button>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- View 2: Modrinth Cloud Hosting -->
		<div v-else class="flex flex-col w-full">
			<ServersManagePageIndex
				:stripe-publishable-key="stripePublishableKey"
				:products="resolvedProducts"
			/>
		</div>

		<!-- Modals -->
		<P2PHostModal ref="p2pHostModal" />
		<P2PJoinModal ref="p2pJoinModal" />
		<TailscaleModal ref="tailscaleModal" />

		<!-- Create Horizon Server Modal -->
		<NewModal
			ref="createServerModal"
			:header="formatMessage(messages.createModalTitle)"
			max-width="480px"
			width="480px"
		>
			<div class="flex w-full flex-col gap-4">
				<label class="flex flex-col gap-1.5 text-xs text-secondary">
					<span class="font-semibold text-contrast">
						{{ formatMessage(messages.serverNameLabel) }}
					</span>
					<Input v-model="newServerName" placeholder="My Horizon Server" />
				</label>

				<label class="flex flex-col gap-1.5 text-xs text-secondary">
					<span class="font-semibold text-contrast">
						{{ formatMessage(messages.serverPortLabel) }}
					</span>
					<Input v-model.number="newServerPort" type="number" min="1024" max="65535" />
				</label>

				<div class="flex justify-end gap-2 pt-2 border-t border-solid border-surface-5">
					<Button type="transparent" @click="createServerModal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</Button>
					<Button
						type="colored"
						color="brand"
						:disabled="!newServerName.trim() || creatingServer"
						@click="handleCreateServer"
					>
						<SpinnerIcon v-if="creatingServer" class="size-4 animate-spin" />
						<PlusIcon v-else class="size-4" />
						{{ formatMessage(messages.createServer) }}
					</Button>
				</div>
			</div>
		</NewModal>
	</div>
</template>
