<template>
	<PageHeader :title="instance.name">
		<template #leading>
			<Avatar
				:src="iconSrc"
				:alt="instance.name"
				size="64px"
				:tint-by="instance.id"
				pad-transparent-corners
			/>
		</template>

		<template
			v-if="instance.shared_instance || instance.quarantined || instance.loader === 'horizon'"
			#badges
		>
			<PageHeaderBadgeItem
				v-if="instance.loader === 'horizon'"
				aria-label="Horizon native runtime"
				class="!border-purple-500 !bg-purple-500/15 !text-purple-400 font-semibold"
			>
				Horizon (Rust)
			</PageHeaderBadgeItem>
			<PageHeaderBadgeItem
				v-if="instance.quarantined"
				:icon="LockIcon"
				aria-label="Locked instance information"
				class="!border-orange !bg-highlight-orange !text-orange"
			>
				Locked
			</PageHeaderBadgeItem>
			<PageHeaderBadgeItem
				v-else-if="instance.shared_instance"
				:tooltip="sharedInstanceTooltip"
				aria-label="Shared instance information"
				class="!border-blue !bg-highlight-blue !text-blue"
			>
				Shared
				<UnknownIcon class="block size-4 shrink-0 text-current" aria-hidden="true" />
			</PageHeaderBadgeItem>
		</template>

		<template #metadata>
			<InstanceHeaderServerMetadata
				v-if="isServerInstance"
				:loading-server-ping="loadingServerPing"
				:players-online="playersOnline"
				:status-online="statusOnline"
				:ping="ping"
				:minecraft-server="minecraftServer"
				:show-instance-play-time="showInstancePlayTime"
				:playtime-label="playtimeLabel ?? formatMessage(messages.neverPlayed)"
			/>
			<PageHeaderMetadata v-else>
				<PageHeaderMetadataItem
					:icon="TagIcon"
					:icon-props="{ tag: loaderDisplayName, enforceType: 'loader' }"
					tooltip="Mod loader and Minecraft version"
				>
					{{ loaderLabel }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataItem
					v-if="instance.loader === 'horizon'"
					:icon="PackageIcon"
					tooltip="Horizon build version (Click to manage)"
					class="cursor-pointer hover:text-contrast"
					@click="versionModal?.show()"
				>
					{{ instance.loader_version || formatMessage(messages.autoVersion) }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataItem
					v-if="showInstancePlayTime && playtimeLabel"
					:icon="TimerIcon"
					tooltip="Total playtime"
				>
					{{ playtimeLabel }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataTimeItem
					v-if="instance.last_played"
					:icon="ClockIcon"
					:date="instance.last_played"
					:label="formatMessage(messages.lastPlayed)"
				/>
				<PageHeaderMetadataItem v-else :icon="ClockIcon" tooltip="Last played">
					{{ formatMessage(messages.neverPlayed) }}
				</PageHeaderMetadataItem>
			</PageHeaderMetadata>
		</template>

		<template #actions>
			<PageHeaderActions>
				<!-- Horizon Mode Toggle: Client vs Dedicated Server -->
				<div
					v-if="instance.loader === 'horizon' && !playing && !isInstalling && !instance.quarantined"
					class="mr-1 flex items-center rounded-xl border border-solid border-surface-5 bg-surface-2 p-0.5 text-xs font-semibold"
				>
					<button
						type="button"
						:class="[
							'flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 transition-all',
							horizonMode === 'client'
								? 'bg-brand text-white shadow-sm'
								: 'text-secondary hover:text-contrast',
						]"
						@click="horizonMode = 'client'"
					>
						<PlayIcon class="size-3.5" />
						{{ formatMessage(messages.modeClient) }}
					</button>
					<button
						type="button"
						:class="[
							'flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 transition-all',
							horizonMode === 'server'
								? 'bg-brand text-white shadow-sm'
								: 'text-secondary hover:text-contrast',
						]"
						@click="horizonMode = 'server'"
					>
						<ServerIcon class="size-3.5" />
						{{ formatMessage(messages.modeServer) }}
					</button>
				</div>

				<Button
					v-if="playing"
					type="colored"
					color="red"
					size="xl"
					native-type="button"
					:disabled="stopping"
					@click="emit('stop')"
				>
					<StopCircleIcon />
					{{
						stopping ? formatMessage(messages.stopping) : formatMessage(commonMessages.stopButton)
					}}
				</Button>
				<Button
					v-else-if="isInstalling"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					disabled
				>
					{{ formatMessage(commonMessages.installingLabel) }}
				</Button>
				<Button
					v-else-if="instance.quarantined"
					v-tooltip="formatMessage(messages.lockedPlayTooltip)"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					disabled
				>
					<PlayIcon />
					{{ formatMessage(commonMessages.playButton) }}
				</Button>
				<Button
					v-else-if="instance.install_stage !== 'installed'"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					@click="emit('repair')"
				>
					<DownloadIcon />
					{{ formatMessage(messages.repair) }}
				</Button>
				<SplitButton
					v-else-if="!loading && isServerInstance"
					type="colored"
					color="brand"
					size="xl"
					:options="serverPlayOptions"
					:menu-label="formatMessage(messages.launchInstance)"
					@click="emit('playServer')"
				>
					<PlayIcon />
					{{ formatMessage(commonMessages.playButton) }}
				</SplitButton>
				<SplitButton
					v-else-if="!loading && instance.loader === 'horizon'"
					type="colored"
					color="brand"
					size="xl"
					:options="horizonPlayOptions"
					:menu-label="formatMessage(messages.launchInstance)"
					@click="handleHorizonPlay"
				>
					<ServerIcon v-if="horizonMode === 'server'" />
					<PlayIcon v-else />
					{{
						horizonMode === 'server'
							? formatMessage(messages.launchServer)
							: formatMessage(commonMessages.playButton)
					}}
				</SplitButton>
				<Button
					v-else-if="!loading"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					@click="emit('play')"
				>
					<PlayIcon />
					{{ formatMessage(commonMessages.playButton) }}
				</Button>
				<Button v-else type="colored" color="brand" size="xl" native-type="button" disabled>{{
					formatMessage(messages.starting)
				}}</Button>

				<IconButton
					v-tooltip="formatMessage(messages.instanceSettings)"
					size="xl"
					:label="formatMessage(messages.instanceSettings)"
					native-type="button"
					@click="emit('settings')"
				>
					<SettingsIcon />
				</IconButton>
				<TeleportOverflowMenu
					type="quiet"
					size="xl"
					:label="formatMessage(messages.moreActions)"
					:tooltip="formatMessage(messages.moreActions)"
					:options="moreActions"
				>
					<MoreVerticalIcon />
				</TeleportOverflowMenu>
			</PageHeaderActions>
		</template>
	</PageHeader>

	<HorizonRequirementsModal ref="requirementsModal" @prepare="handleRequirementsPrepare" />
	<HorizonVersionModal ref="versionModal" :instance="instance" />
</template>

<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	ClockIcon,
	DownloadIcon,
	ExternalIcon,
	FolderOpenIcon,
	LockIcon,
	MoreVerticalIcon,
	PackageIcon,
	PlayIcon,
	ReportIcon,
	ServerIcon,
	SettingsIcon,
	ShieldCheckIcon,
	StopCircleIcon,
	TagIcon,
	TimerIcon,
	UnknownIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { Button, IconButton, SplitButton, TeleportOverflowMenu } from '@modrinth/ui'
import {
	Avatar,
	type ButtonMenuOption,
	commonMessages,
	defineMessages,
	formatLoaderLabel,
	PageHeader,
	PageHeaderActions,
	PageHeaderBadgeItem,
	PageHeaderMetadata,
	PageHeaderMetadataItem,
	PageHeaderMetadataTimeItem,
	type ServerLoader,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import HorizonRequirementsModal from '@/components/ui/modal/HorizonRequirementsModal.vue'
import HorizonVersionModal from '@/components/ui/modal/HorizonVersionModal.vue'
import type { GameInstance } from '@/helpers/types'

import InstanceHeaderServerMetadata from './instance-page-header-server-metadata.vue'

const messages = defineMessages({
	createShortcut: {
		id: 'instance.action.create-shortcut',
		defaultMessage: 'Create shortcut',
	},
	exportModpack: {
		id: 'instance.action.export-modpack',
		defaultMessage: 'Export modpack',
	},
	instanceSettings: {
		id: 'instance.action.settings',
		defaultMessage: 'Instance settings',
	},
	launchInstance: {
		id: 'instance.action.launch-instance',
		defaultMessage: 'Launch instance',
	},
	moreActions: {
		id: 'instance.action.more-actions',
		defaultMessage: 'More actions',
	},
	neverPlayed: {
		id: 'instance.playtime.never-played',
		defaultMessage: 'Never played',
	},
	lastPlayed: {
		id: 'instance.last-played',
		defaultMessage: 'Last played',
	},
	openFolder: {
		id: 'instance.action.open-folder',
		defaultMessage: 'Open folder',
	},
	repair: {
		id: 'instance.action.repair',
		defaultMessage: 'Repair',
	},
	lockedPlayTooltip: {
		id: 'instance.locked.play-tooltip',
		defaultMessage: 'This instance has been locked',
	},
	starting: {
		id: 'instance.action.starting',
		defaultMessage: 'Starting...',
	},
	stopping: {
		id: 'instance.action.stopping',
		defaultMessage: 'Stopping...',
	},
	sharedInstanceTooltip: {
		id: 'instance.shared-instance.tooltip',
		defaultMessage: "This instance's content is being managed by someone else.",
	},
	sharedInstanceOwnerTooltip: {
		id: 'instance.shared-instance.owner-tooltip',
		defaultMessage: "This instance's content is being shared to other users.",
	},
	launchClient: {
		id: 'instance.action.launch-client',
		defaultMessage: 'Launch Horizon Client',
	},
	launchServer: {
		id: 'instance.action.launch-server',
		defaultMessage: 'Launch Horizon Server',
	},
	checkRequirements: {
		id: 'instance.action.check-requirements',
		defaultMessage: 'Check Horizon Requirements',
	},
	prepareResources: {
		id: 'instance.action.prepare-resources',
		defaultMessage: 'Prepare Game Files',
	},
	selectVersion: {
		id: 'instance.action.select-version',
		defaultMessage: 'Select Horizon Version',
	},
	modeClient: {
		id: 'app.horizon.mode.client',
		defaultMessage: 'Client',
	},
	modeServer: {
		id: 'app.horizon.mode.server',
		defaultMessage: 'Dedicated Server',
	},
	autoVersion: {
		id: 'app.horizon.version.auto',
		defaultMessage: 'Auto (Latest Installed)',
	},
})

const props = withDefaults(
	defineProps<{
		instance: GameInstance
		iconSrc?: string | null
		isServerInstance?: boolean
		showInstancePlayTime?: boolean
		timePlayed?: number
		playing?: boolean
		loading?: boolean
		stopping?: boolean
		loadingServerPing?: boolean
		playersOnline?: number
		statusOnline?: boolean
		ping?: number
		minecraftServer?: Labrinth.Projects.v3.Project['minecraft_server']
	}>(),
	{
		iconSrc: null,
		isServerInstance: false,
		showInstancePlayTime: false,
		timePlayed: 0,
		playing: false,
		loading: false,
		stopping: false,
		loadingServerPing: false,
		playersOnline: undefined,
		statusOnline: false,
		ping: undefined,
		minecraftServer: undefined,
	},
)

const emit = defineEmits<{
	repair: []
	stop: []
	play: []
	playServer: []
	prepareHorizon: []
	settings: []
	openFolder: []
	export: []
	createShortcut: []
	report: [event?: MouseEvent]
}>()

const installingStages = ['installing', 'pack_installing', 'minecraft_installing']

const { formatMessage } = useVIntl()

const isInstalling = computed(() => installingStages.includes(props.instance.install_stage))

const loaderDisplayName = computed(() => formatLoaderLabel(props.instance.loader) as ServerLoader)
const loaderLabel = computed(() =>
	[loaderDisplayName.value, props.instance.game_version].filter(Boolean).join(' '),
)
const sharedInstanceTooltip = computed(() =>
	formatMessage(
		props.instance.shared_instance?.role === 'owner'
			? messages.sharedInstanceOwnerTooltip
			: messages.sharedInstanceTooltip,
	),
)
const playtimeLabel = computed(() => {
	const seconds = Math.floor(props.timePlayed)
	if (seconds <= 0) {
		return undefined
	}

	const hours = Math.floor(seconds / 3600)
	if (hours >= 1) {
		return `${hours} hour${hours > 1 ? 's' : ''}`
	}

	const minutes = Math.floor(seconds / 60)
	if (minutes >= 1) {
		return `${minutes} minute${minutes > 1 ? 's' : ''}`
	}

	return `${seconds} second${seconds === 1 ? '' : 's'}`
})
const serverPlayOptions = computed<ButtonMenuOption[]>(() => [
	{
		id: 'launch_instance',
		label: formatMessage(messages.launchInstance),
		icon: PlayIcon,
		action: () => emit('play'),
	},
])
const requirementsModal = ref<InstanceType<typeof HorizonRequirementsModal> | null>(null)
const versionModal = ref<InstanceType<typeof HorizonVersionModal> | null>(null)
const horizonMode = ref<'client' | 'server'>('client')

function handleHorizonPlay() {
	if (horizonMode.value === 'server') {
		emit('playServer')
	} else {
		emit('play')
	}
}

const horizonPlayOptions = computed<ButtonMenuOption[]>(() => [
	{
		id: 'launch_client',
		label: formatMessage(messages.launchClient),
		icon: PlayIcon,
		action: () => emit('play'),
	},
	{
		id: 'launch_server',
		label: formatMessage(messages.launchServer),
		icon: ServerIcon,
		action: () => emit('playServer'),
	},
	{ type: 'divider' },
	{
		id: 'select_version',
		label: formatMessage(messages.selectVersion),
		icon: TagIcon,
		action: () => versionModal.value?.show(),
	},
	{
		id: 'prepare_resources',
		label: formatMessage(messages.prepareResources),
		icon: WrenchIcon,
		action: () => emit('prepareHorizon'),
	},
	{
		id: 'check_requirements',
		label: formatMessage(messages.checkRequirements),
		icon: ShieldCheckIcon,
		action: () => requirementsModal.value?.show(),
	},
])

function handleRequirementsPrepare() {
	requirementsModal.value?.hide()
	emit('prepareHorizon')
}
const moreActions = computed<ButtonMenuOption[]>(() => {
	const actions: ButtonMenuOption[] = [
		{
			id: 'open-folder',
			label: formatMessage(messages.openFolder),
			icon: FolderOpenIcon,
			action: () => emit('openFolder'),
		},
	]

	if (!props.instance.quarantined) {
		actions.push(
			{
				id: 'export-mrpack',
				label: formatMessage(messages.exportModpack),
				icon: PackageIcon,
				action: () => emit('export'),
			},
			{
				id: 'create-shortcut',
				label: formatMessage(messages.createShortcut),
				icon: ExternalIcon,
				action: () => emit('createShortcut'),
			},
		)
	}

	if (props.instance.shared_instance?.role === 'member') {
		actions.push(
			{ type: 'divider' },
			{
				id: 'report-shared-instance',
				label: formatMessage(commonMessages.reportButton),
				icon: ReportIcon,
				tone: 'red',
				action: (event) => emit('report', event),
			},
		)
	}

	return actions
})
</script>
