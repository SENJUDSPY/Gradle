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
			v-if="instance.shared_instance || instance.quarantined || activeRuntime === 'horizon' || instance.loader === 'horizon'"
			#badges
		>
			<PageHeaderBadgeItem
				v-if="activeRuntime === 'horizon' || instance.loader === 'horizon'"
				aria-label="Gradle Rust Edition"
				class="!border-brand-accent/50 !bg-brand-accent/15 !text-brand-accent font-semibold shadow-sm"
			>
				Gradle Rust Edition
			</PageHeaderBadgeItem>
			<PageHeaderBadgeItem
				v-else
				aria-label="Java Edition"
				class="!border-orange/50 !bg-orange/15 !text-orange font-semibold shadow-sm"
			>
				Java Edition
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
					tooltip="Gradle build version (Click to manage)"
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
				<!-- Runtime Switch (Segmented Control): Horizon (Rust, optimized) vs Java (classic) -->
				<SegmentedControl
					v-if="!playing && !isInstalling && !instance.quarantined"
					:model-value="activeRuntime"
					:items="['horizon', 'java']"
					:format-label="(item) => (item === 'horizon' ? formatMessage(messages.runtimeHorizon) : formatMessage(messages.runtimeJava))"
					:aria-label="formatMessage(messages.runtimeSwitchLabel)"
					class="mr-2"
					@update:model-value="(val) => val && handleRuntimeChange(val as InstanceRuntime)"
				/>

				<!-- Update Available Badge (non-intrusive) -->
				<button
					v-if="activeRuntime === 'horizon' && hasHorizonUpdate && !playing"
					type="button"
					class="mr-2 flex items-center gap-1.5 rounded-xl border border-solid border-brand/30 bg-brand/10 px-2.5 py-1.5 text-xs font-semibold text-brand transition-all hover:bg-brand/20 shadow-sm cursor-pointer"
					:title="`Update available: ${latestReleaseTag}`"
					@click="versionModal?.show()"
				>
					<SparklesIcon class="size-3.5" />
					<span>{{ formatMessage(messages.updateAvailable) }}</span>
				</button>

				<!-- Horizon Mode Toggle: Client vs Dedicated Server -->
				<div
					v-if="activeRuntime === 'horizon' && !playing && !isInstalling && !instance.quarantined"
					class="mr-1 flex items-center rounded-xl border border-solid border-surface-5 bg-surface-2 p-0.5 text-xs font-semibold shadow-sm"
				>
					<button
						type="button"
						:class="[
							'flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 transition-all duration-150',
							horizonMode === 'client'
								? 'bg-gradient-to-r from-[#5b5bf0] to-[#22d3ee] text-white shadow-sm'
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
							'flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 transition-all duration-150',
							horizonMode === 'server'
								? 'bg-gradient-to-r from-[#5b5bf0] to-[#22d3ee] text-white shadow-sm'
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
				<div
					v-else-if="!loading && isUnsupportedNativeVersion"
					class="flex items-center gap-2"
				>
					<div
						class="flex items-center gap-2 rounded-xl border border-solid border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-300 font-medium"
					>
						<TriangleAlertIcon class="size-4 shrink-0 text-amber-400" />
						<span>{{ formatMessage(messages.nativeVersionMismatch, { version: instance.game_version }) }}</span>
					</div>
					<Button
						type="colored"
						color="brand"
						size="xl"
						native-type="button"
						:disabled="converting"
						@click="handleConvertToNative"
					>
						<WrenchIcon />
						{{ formatMessage(messages.convertToNative) }}
					</Button>
					<Button
						type="outlined"
						size="xl"
						native-type="button"
						@click="handleRuntimeChange('java')"
					>
						{{ formatMessage(messages.useJavaEdition) }}
					</Button>
				</div>
				<SplitButton
					v-else-if="!loading"
					type="colored"
					color="brand"
					size="xl"
					:options="dualPlayOptions"
					:menu-label="formatMessage(messages.launchInstance)"
					@click="handleHorizonPlay"
				>
					<ServerIcon v-if="activeRuntime === 'horizon' && horizonMode === 'server'" />
					<PlayIcon v-else />
					{{
						activeRuntime === 'horizon' && horizonMode === 'server'
							? formatMessage(messages.launchServer)
							: activeRuntime === 'horizon'
								? formatMessage(messages.playHorizon)
								: formatMessage(messages.playJava)
					}}
				</SplitButton>
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

	<HorizonRequirementsModal
		ref="requirementsModal"
		@prepare="handleRequirementsPrepare"
		@install-runtime="versionModal?.show()"
	/>
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
	SparklesIcon,
	StopCircleIcon,
	TagIcon,
	TimerIcon,
	TriangleAlertIcon,
	UnknownIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { Button, IconButton, SegmentedControl, SplitButton, TeleportOverflowMenu } from '@modrinth/ui'
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
import { computed, onMounted, ref } from 'vue'

import HorizonRequirementsModal from '@/components/ui/modal/HorizonRequirementsModal.vue'
import HorizonVersionModal from '@/components/ui/modal/HorizonVersionModal.vue'
import {
	getHorizonReleases,
	getInstalledHorizonVersions,
	type HorizonRelease,
} from '@/helpers/horizon'
import { edit, type InstanceRuntime } from '@/helpers/instance'
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
	runtimeHorizon: {
		id: 'instance.runtime.horizon',
		defaultMessage: 'Rust Edition (Native)',
	},
	runtimeJava: {
		id: 'instance.runtime.java',
		defaultMessage: 'Java Edition (Classic)',
	},
	updateAvailable: {
		id: 'instance.runtime.update-available',
		defaultMessage: 'Update available',
	},
	playHorizon: {
		id: 'instance.action.play-horizon',
		defaultMessage: 'Play with Gradle',
	},
	playJava: {
		id: 'instance.action.play-java',
		defaultMessage: 'Play with Java',
	},
	runtimeSwitchLabel: {
		id: 'instance.runtime.switch-label',
		defaultMessage: 'Launch runtime',
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
		defaultMessage: 'Launch Gradle Client',
	},
	launchServer: {
		id: 'instance.action.launch-server',
		defaultMessage: 'Launch Gradle Server',
	},
	checkRequirements: {
		id: 'instance.action.check-requirements',
		defaultMessage: 'Check Gradle Requirements',
	},
	prepareResources: {
		id: 'instance.action.prepare-resources',
		defaultMessage: 'Prepare Game Files',
	},
	selectVersion: {
		id: 'instance.action.select-version',
		defaultMessage: 'Select Gradle Version',
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
	runDiagnostics: {
		id: 'app.horizon.diagnostics.run',
		defaultMessage: 'Run Diagnostics',
	},
	convertToNative: {
		id: 'instance.action.convert-native',
		defaultMessage: 'Convert to 26.3',
	},
	useJavaEdition: {
		id: 'instance.action.use-java-edition',
		defaultMessage: 'Use Java Edition',
	},
	nativeVersionMismatch: {
		id: 'instance.notice.native-version-mismatch',
		defaultMessage: 'Gradle Rust Edition requires Minecraft 26.3 (current: {version}).',
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
		runtime?: InstanceRuntime
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
		runtime: undefined,
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
	'update:runtime': [runtime: InstanceRuntime]
	playRuntime: [runtime: InstanceRuntime]
	refresh: []
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
const activeRuntime = computed<InstanceRuntime>({
	get: () => props.runtime ?? props.instance.runtime ?? 'horizon',
	set: (val: InstanceRuntime) => emit('update:runtime', val),
})

function handleRuntimeChange(val: InstanceRuntime) {
	emit('update:runtime', val)
}

const isRustEdition = computed(
	() => activeRuntime.value === 'horizon' || props.instance.loader === 'horizon',
)
const isUnsupportedNativeVersion = computed(
	() => isRustEdition.value && props.instance.game_version !== '26.3',
)

const converting = ref(false)
async function handleConvertToNative() {
	try {
		converting.value = true
		await edit(props.instance.id, { game_version: '26.3' })
		emit('refresh')
	} catch (err) {
		console.error('Failed to convert instance to 26.3:', err)
	} finally {
		converting.value = false
	}
}

const requirementsModal = ref<InstanceType<typeof HorizonRequirementsModal> | null>(null)
const versionModal = ref<InstanceType<typeof HorizonVersionModal> | null>(null)
const horizonMode = ref<'client' | 'server'>('client')

const releases = ref<HorizonRelease[]>([])
const installedVersions = ref<string[]>([])

onMounted(async () => {
	try {
		const [relList, instList] = await Promise.all([
			getHorizonReleases().catch(() => []),
			getInstalledHorizonVersions().catch(() => []),
		])
		releases.value = relList
		installedVersions.value = instList
	} catch (e) {
		console.warn('Failed to check Horizon updates:', e)
	}
})

const latestReleaseTag = computed(() => {
	if (releases.value.length === 0) return null
	const stable = releases.value.find((r) => !r.prerelease) ?? releases.value[0]
	return stable?.tag ?? null
})

const hasHorizonUpdate = computed(() => {
	const latest = latestReleaseTag.value
	if (!latest) return false
	const current = props.instance.loader_version || installedVersions.value[0]
	if (!current) return false
	return current !== latest
})

function handleHorizonPlay() {
	if (isUnsupportedNativeVersion.value && activeRuntime.value === 'horizon') {
		return
	}
	if (activeRuntime.value === 'horizon') {
		if (horizonMode.value === 'server') {
			emit('playServer')
		} else {
			emit('playRuntime', 'horizon')
		}
	} else {
		emit('playRuntime', 'java')
	}
}

const dualPlayOptions = computed<ButtonMenuOption[]>(() => {
	const options: ButtonMenuOption[] = [
		{
			id: 'play_horizon',
			label: formatMessage(messages.playHorizon),
			icon: PlayIcon,
			disabled: isUnsupportedNativeVersion.value,
			action: () => {
				if (isUnsupportedNativeVersion.value) return
				handleRuntimeChange('horizon')
				emit('playRuntime', 'horizon')
			},
		},
		{
			id: 'play_java',
			label: formatMessage(messages.playJava),
			icon: PlayIcon,
			action: () => {
				handleRuntimeChange('java')
				emit('playRuntime', 'java')
			},
		},
	]

	if (activeRuntime.value === 'horizon') {
		options.push(
			{ type: 'divider' },
			{
				id: 'launch_server',
				label: formatMessage(messages.launchServer),
				icon: ServerIcon,
				action: () => emit('playServer'),
			},
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
		)
	} else if (props.isServerInstance) {
		options.push(
			{ type: 'divider' },
			{
				id: 'launch_server_project',
				label: formatMessage(messages.launchInstance),
				icon: ServerIcon,
				action: () => emit('playServer'),
			},
		)
	}

	return options
})

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

		if (activeRuntime.value === 'horizon') {
			actions.push({
				id: 'horizon-diagnostics',
				label: formatMessage(messages.runDiagnostics),
				icon: WrenchIcon,
				action: () => {
					requirementsModal.value?.show()
					requirementsModal.value?.switchToDiagnostics()
				},
			})
		}
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
