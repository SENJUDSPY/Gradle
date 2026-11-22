<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="!isBusy"
		max-width="640px"
		width="640px"
		:on-hide="handleHide"
	>
		<div class="flex w-full flex-col gap-5">
			<!-- Intro Card -->
			<div class="flex items-start gap-3.5 rounded-2xl border border-solid border-brand/30 bg-brand/10 p-4 text-contrast">
				<div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand text-white shadow-sm">
					<SparklesIcon class="size-5" />
				</div>
				<div class="flex flex-col gap-1">
					<span class="text-sm font-bold text-contrast">
						{{ formatMessage(messages.welcomeHeading) }}
					</span>
					<span class="text-xs text-secondary leading-relaxed">
						{{ formatMessage(messages.welcomeBody) }}
					</span>
				</div>
			</div>

			<!-- Reserved Error Container to avoid modal jumping -->
			<div class="min-h-[56px] flex flex-col justify-center">
				<div
					v-if="formattedError"
					class="flex flex-col gap-1.5 rounded-xl border border-solid border-red-500/30 bg-red-500/10 p-3 text-xs text-red-400"
				>
					<div class="flex items-start justify-between gap-2">
						<div class="flex flex-col gap-0.5">
							<div class="flex items-center gap-1.5 font-semibold">
								<span
									v-if="formattedError.code"
									class="rounded bg-red-500/20 px-1.5 py-0.5 font-mono text-[10px] text-red-300"
								>
									{{ formattedError.code }}
								</span>
								<span>{{ formattedError.message }}</span>
							</div>
							<span v-if="formattedError.hint" class="text-[11px] text-secondary">
								{{ formattedError.hint }}
							</span>
						</div>
						<button
							type="button"
							class="shrink-0 rounded border border-solid border-red-500/30 bg-red-500/20 px-2 py-1 text-[11px] font-medium text-red-300 hover:bg-red-500/30 transition-colors cursor-pointer outline-none ring-0 focus:outline-none"
							@click="copyErrorDetails"
						>
							{{ copiedError ? 'Copied!' : 'Copy details' }}
						</button>
					</div>
				</div>
			</div>

			<!-- Step 1: Runtime Engine -->
			<div
				:class="[
					'flex flex-col gap-3 rounded-2xl border border-solid p-4 transition-colors',
					hasInstalledRuntime
						? 'border-green-500/30 bg-green-500/5'
						: isInstalling
						? 'border-brand/40 bg-surface-2'
						: 'border-surface-5 bg-surface-2',
				]"
			>
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<div
							:class="[
								'flex size-7 shrink-0 items-center justify-center rounded-full text-xs font-bold',
								hasInstalledRuntime
									? 'bg-green-500 text-white'
									: 'bg-surface-4 text-secondary',
							]"
						>
							<CheckIcon v-if="hasInstalledRuntime" class="size-4" />
							<span v-else>1</span>
						</div>
						<div class="flex flex-col gap-0.5">
							<div class="flex items-center gap-2">
								<span class="text-sm font-semibold text-contrast">
									{{ formatMessage(messages.step1Title) }}
								</span>
								<span
									v-if="hasInstalledRuntime"
									class="rounded bg-green-500/20 px-2 py-0.5 text-[10px] font-semibold text-green-400"
								>
									{{ formatMessage(messages.installed) }}
								</span>
							</div>
							<span class="text-xs text-secondary">
								{{
									hasInstalledRuntime
										? `Active engine: ${installedVersions[0]}`
										: formatMessage(messages.step1Subtitle)
								}}
							</span>
						</div>
					</div>

					<Button
						v-if="!hasInstalledRuntime"
						size="sm"
						type="colored"
						color="brand"
						:disabled="isBusy || !targetRelease"
						@click="installRecommendedRuntime"
					>
						<SpinnerIcon v-if="isInstalling" class="size-4 animate-spin" />
						<DownloadIcon v-else class="size-4" />
						{{ isInstalling ? formatMessage(messages.installing) : formatMessage(messages.installButton) }}
					</Button>
				</div>

				<!-- Download Progress Bar -->
				<div v-if="isInstalling && downloadProgress" class="flex flex-col gap-1.5 pt-1">
					<div class="flex items-center justify-between text-xs">
						<span class="font-medium text-brand">
							{{
								downloadProgress.stage === 'verifying'
									? formatMessage(messages.verifyingChecksum)
									: downloadProgress.stage === 'extracting'
									? formatMessage(messages.extractingBinaries)
									: downloadProgress.stage === 'done'
									? formatMessage(messages.installComplete)
									: formatMessage(messages.downloadingRuntime)
							}}
						</span>
						<span class="font-mono text-secondary">
							{{ Math.round((downloadProgress.percent || 0) * 100) }}%
							<template v-if="downloadProgress.speed_bps > 0">
								• {{ (downloadProgress.speed_bps / 1048576).toFixed(1) }} MB/s
							</template>
						</span>
					</div>
					<div class="h-2 w-full overflow-hidden rounded-full bg-surface-4">
						<div
							class="h-full bg-brand transition-all duration-150 rounded-full"
							:style="{ width: `${Math.round((downloadProgress.percent || 0) * 100)}%` }"
						/>
					</div>
					<div v-if="downloadProgress.total" class="flex justify-between text-[11px] text-secondary">
						<span>{{ (downloadProgress.downloaded / 1048576).toFixed(1) }} MB / {{ (downloadProgress.total / 1048576).toFixed(1) }} MB</span>
						<span>SHA-256 Verified HTTPS</span>
					</div>
				</div>
			</div>

			<!-- Step 2: Asset Preparation -->
			<div
				:class="[
					'flex flex-col gap-3 rounded-2xl border border-solid p-4 transition-colors',
					isResourcesPrepared
						? 'border-green-500/30 bg-green-500/5'
						: isPreparingResources
						? 'border-brand/40 bg-surface-2'
						: 'border-surface-5 bg-surface-2',
				]"
			>
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-3">
						<div
							:class="[
								'flex size-7 shrink-0 items-center justify-center rounded-full text-xs font-bold',
								isResourcesPrepared
									? 'bg-green-500 text-white'
									: 'bg-surface-4 text-secondary',
							]"
						>
							<CheckIcon v-if="isResourcesPrepared" class="size-4" />
							<span v-else>2</span>
						</div>
						<div class="flex flex-col gap-0.5">
							<div class="flex items-center gap-2">
								<span class="text-sm font-semibold text-contrast">
									{{ formatMessage(messages.step2Title) }}
								</span>
								<span
									v-if="isResourcesPrepared"
									class="rounded bg-green-500/20 px-2 py-0.5 text-[10px] font-semibold text-green-400"
								>
									{{ formatMessage(messages.ready) }}
								</span>
							</div>
							<span class="text-xs text-secondary">
								{{
									isResourcesPrepared
										? formatMessage(messages.step2Done)
										: formatMessage(messages.step2Subtitle)
								}}
							</span>
						</div>
					</div>

					<div class="flex items-center gap-2">
						<Button
							v-if="isPreparingResources"
							size="sm"
							type="outlined"
							@click="cancelAssetPreparation"
						>
							{{ formatMessage(messages.cancel) }}
						</Button>
						<Button
							v-else-if="!isResourcesPrepared"
							size="sm"
							type="colored"
							color="brand"
							:disabled="isBusy || !hasInstalledRuntime"
							@click="prepareAssets"
						>
							<WrenchIcon class="size-4" />
							{{ formatMessage(messages.prepareButton) }}
						</Button>
					</div>
				</div>

				<!-- Preparation Progress -->
				<div v-if="isPreparingResources && prepareProgress" class="flex flex-col gap-1.5 pt-1">
					<div class="flex items-center justify-between text-xs">
						<span class="font-medium text-brand">{{ prepareProgress.stage }}</span>
						<span class="font-mono text-secondary">{{ Math.round((prepareProgress.percent || 0) * 100) }}%</span>
					</div>
					<div class="h-2 w-full overflow-hidden rounded-full bg-surface-4">
						<div
							class="h-full bg-brand transition-all duration-150 rounded-full"
							:style="{ width: `${Math.round((prepareProgress.percent || 0) * 100)}%` }"
						/>
					</div>
					<span class="text-[11px] text-secondary truncate">{{ prepareProgress.details }}</span>
				</div>
			</div>

			<!-- Success Banner & Launch CTA -->
			<div
				v-if="hasInstalledRuntime && isResourcesPrepared"
				class="flex items-center justify-between rounded-2xl border border-solid border-green-500/40 bg-green-500/10 p-4"
			>
				<div class="flex items-center gap-3">
					<CheckCircleIcon class="size-6 text-green-400 shrink-0" />
					<div class="flex flex-col gap-0.5">
						<span class="text-sm font-bold text-contrast">
							{{ formatMessage(messages.allSetHeading) }}
						</span>
						<span class="text-xs text-secondary">
							{{ formatMessage(messages.allSetBody) }}
						</span>
					</div>
				</div>

				<Button size="lg" type="colored" color="brand" @click="handleLaunch">
					<PlayIcon class="size-5" />
					{{ formatMessage(messages.launchNowButton) }}
				</Button>
			</div>

			<!-- Footer Options -->
			<div class="flex items-center justify-between border-t border-solid border-surface-5 pt-3">
				<Button
					type="quiet"
					:disabled="isBusy"
					class="!outline-none !ring-0 focus:!outline-none focus:!ring-0 shadow-none"
					@click="handleSwitchToJava"
				>
					{{ formatMessage(messages.switchToJavaButton) }}
				</Button>

				<Button
					type="outlined"
					:disabled="isBusy"
					class="!outline-none !ring-0 focus:!outline-none focus:!ring-0 shadow-none"
					@click="hide"
				>
					{{ formatMessage(commonMessages.closeButton) }}
				</Button>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckCircleIcon,
	CheckIcon,
	DownloadIcon,
	PlayIcon,
	SparklesIcon,
	SpinnerIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref } from 'vue'

import {
	cancelHorizonPreparation,
	type DownloadProgress,
	getHorizonReleases,
	getInstalledHorizonVersions,
	type HorizonRelease,
	installHorizonVersion,
	isHorizonResourcesPrepared,
	onHorizonDownloadProgress,
	onHorizonPrepareProgress,
	type PreparationProgress,
	prepareHorizonResources,
} from '@/helpers/horizon'
import type { GameInstance } from '@/helpers/types'

const props = defineProps<{
	instance?: GameInstance | null
}>()

const emit = defineEmits<{
	(e: 'completed'): void
	(e: 'switchToJava'): void
}>()

const messages = defineMessages({
	title: {
		id: 'app.horizon.setup.title',
		defaultMessage: 'Gradle Rust Edition Setup',
	},
	welcomeHeading: {
		id: 'app.horizon.setup.welcome-heading',
		defaultMessage: 'Welcome to Gradle Rust Edition',
	},
	welcomeBody: {
		id: 'app.horizon.setup.welcome-body',
		defaultMessage:
			'Experience Minecraft 26.3 with native Rust performance, instantaneous chunk meshing, and modern wgpu graphics. Complete this quick one-time setup to start playing.',
	},
	step1Title: {
		id: 'app.horizon.setup.step1-title',
		defaultMessage: 'Native Engine Runtime',
	},
	step1Subtitle: {
		id: 'app.horizon.setup.step1-subtitle',
		defaultMessage: 'Download the standalone Rust client and server binaries.',
	},
	step2Title: {
		id: 'app.horizon.setup.step2-title',
		defaultMessage: 'Minecraft 26.3 Assets',
	},
	step2Subtitle: {
		id: 'app.horizon.setup.step2-subtitle',
		defaultMessage: 'Prepare game textures, models, and sounds from your Minecraft installation.',
	},
	step2Done: {
		id: 'app.horizon.setup.step2-done',
		defaultMessage: 'Genuine Minecraft 26.3 resources extracted and verified.',
	},
	installed: {
		id: 'app.horizon.setup.installed',
		defaultMessage: 'Installed',
	},
	ready: {
		id: 'app.horizon.setup.ready',
		defaultMessage: 'Ready',
	},
	installButton: {
		id: 'app.horizon.setup.install-button',
		defaultMessage: 'Install Engine',
	},
	installing: {
		id: 'app.horizon.setup.installing',
		defaultMessage: 'Installing...',
	},
	prepareButton: {
		id: 'app.horizon.setup.prepare-button',
		defaultMessage: 'Prepare Assets',
	},
	preparing: {
		id: 'app.horizon.setup.preparing',
		defaultMessage: 'Preparing...',
	},
	verifyingChecksum: {
		id: 'app.horizon.setup.verifying-checksum',
		defaultMessage: 'Verifying SHA-256 Checksum...',
	},
	extractingBinaries: {
		id: 'app.horizon.setup.extracting-binaries',
		defaultMessage: 'Extracting binaries...',
	},
	installComplete: {
		id: 'app.horizon.setup.install-complete',
		defaultMessage: 'Installation complete!',
	},
	downloadingRuntime: {
		id: 'app.horizon.setup.downloading-runtime',
		defaultMessage: 'Downloading engine runtime...',
	},
	allSetHeading: {
		id: 'app.horizon.setup.all-set-heading',
		defaultMessage: "You're all set!",
	},
	allSetBody: {
		id: 'app.horizon.setup.all-set-body',
		defaultMessage: 'Gradle Rust Edition runtime and assets are ready.',
	},
	launchNowButton: {
		id: 'app.horizon.setup.launch-now-button',
		defaultMessage: 'Launch Game',
	},
	switchToJavaButton: {
		id: 'app.horizon.setup.switch-to-java-button',
		defaultMessage: 'Play Java Edition Instead',
	},
	cancel: {
		id: 'app.horizon.preparation.cancel',
		defaultMessage: 'Cancel',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal>>()

const releases = ref<HorizonRelease[]>([])
const installedVersions = ref<string[]>([])
const isResourcesPrepared = ref(false)

const isInstalling = ref(false)
const isPreparingResources = ref(false)
const downloadProgress = ref<DownloadProgress | null>(null)
const prepareProgress = ref<PreparationProgress | null>(null)

interface FormattedError {
	message: string
	code?: string
	hint?: string
	raw?: any
}

const formattedError = ref<FormattedError | null>(null)
const copiedError = ref(false)

function setAppError(e: any, contextHint?: string) {
	console.error('[HorizonSetup] Operation error:', e)
	if (!e) {
		formattedError.value = null
		return
	}
	let message = 'An unexpected error occurred'
	let code: string | undefined
	let hint: string | undefined = contextHint

	if (typeof e === 'string') {
		message = e
		if (e.includes('not allowed by ACL')) {
			code = 'ACL_PERMISSION_DENIED'
			hint = 'Command execution was blocked by application security rules.'
		}
	} else if (typeof e === 'object') {
		message = e.message || e.error || (typeof e.toString === 'function' && e.toString() !== '[object Object]' ? e.toString() : 'An error occurred during operation')
		code = e.code
		hint = e.hint || hint
		if (typeof message === 'string' && message.includes('not allowed by ACL')) {
			code = code || 'ACL_PERMISSION_DENIED'
			hint = hint || 'Command execution was blocked by application security rules.'
		}
	}
	formattedError.value = {
		message,
		code,
		hint,
		raw: e,
	}
}

function clearAppError() {
	formattedError.value = null
	copiedError.value = false
}

async function copyErrorDetails() {
	if (!formattedError.value) return
	try {
		const payload = {
			code: formattedError.value.code,
			message: formattedError.value.message,
			hint: formattedError.value.hint,
			details: formattedError.value.raw,
			timestamp: new Date().toISOString(),
		}
		await navigator.clipboard.writeText(JSON.stringify(payload, null, 2))
		copiedError.value = true
		setTimeout(() => {
			copiedError.value = false
		}, 2000)
	} catch (err) {
		console.error('Failed to copy error details:', err)
	}
}

let onLaunchCallback: (() => void) | null = null
let onSwitchToJavaCallback: (() => void) | null = null

let unlistenDownload: (() => void) | null = null
let unlistenPrepare: (() => void) | null = null

const hasInstalledRuntime = computed(() => installedVersions.value.length > 0)
const isBusy = computed(() => isInstalling.value || isPreparingResources.value)
const targetRelease = computed<HorizonRelease | null>(() => {
	if (releases.value.length === 0) return null
	const stable = releases.value.find((r) => !r.prerelease)
	return stable ?? releases.value[0]
})

onMounted(async () => {
	unlistenDownload = await onHorizonDownloadProgress((prog) => {
		downloadProgress.value = prog
	})
	unlistenPrepare = await onHorizonPrepareProgress((prog) => {
		prepareProgress.value = prog
	})
})

onUnmounted(() => {
	unlistenDownload?.()
	unlistenPrepare?.()
})

async function refreshState() {
	clearAppError()
	try {
		const [relList, instList, prep] = await Promise.all([
			getHorizonReleases(),
			getInstalledHorizonVersions(),
			isHorizonResourcesPrepared(),
		])
		releases.value = relList
		installedVersions.value = instList
		isResourcesPrepared.value = prep
	} catch (e) {
		setAppError(e, 'Failed to fetch engine runtime versions or asset preparation status.')
	}
}

async function installRecommendedRuntime() {
	const rel = targetRelease.value
	if (!rel) return
	isInstalling.value = true
	clearAppError()
	try {
		await installHorizonVersion(rel.tag)
		await refreshState()
		// Auto-trigger asset preparation if not yet prepared
		if (!isResourcesPrepared.value) {
			await prepareAssets()
		}
	} catch (e) {
		setAppError(e, 'Failed to download and verify the native engine release.')
	} finally {
		isInstalling.value = false
	}
}

async function prepareAssets() {
	isPreparingResources.value = true
	clearAppError()
	try {
		await prepareHorizonResources()
		await refreshState()
	} catch (e) {
		setAppError(e, 'Failed to extract assets from genuine Minecraft jar.')
	} finally {
		isPreparingResources.value = false
	}
}

async function cancelAssetPreparation() {
	try {
		await cancelHorizonPreparation()
	} catch (e) {
		console.error('Failed to cancel asset preparation:', e)
	} finally {
		isPreparingResources.value = false
		setAppError('Asset preparation cancelled by user', 'You can restart asset preparation anytime.')
	}
}

function handleLaunch() {
	const cb = onLaunchCallback
	onLaunchCallback = null
	hide()
	emit('completed')
	cb?.()
}

function handleSwitchToJava() {
	const cb = onSwitchToJavaCallback
	onSwitchToJavaCallback = null
	hide()
	emit('switchToJava')
	cb?.()
}

function handleHide() {
	// Clean up callbacks if user closed modal
	onLaunchCallback = null
	onSwitchToJavaCallback = null
}

function show(options?: { onLaunch?: () => void; onSwitchToJava?: () => void }) {
	onLaunchCallback = options?.onLaunch ?? null
	onSwitchToJavaCallback = options?.onSwitchToJava ?? null
	clearAppError()
	downloadProgress.value = null
	prepareProgress.value = null
	modalRef.value?.show()
	void refreshState()
}

function hide() {
	modalRef.value?.hide()
}

defineExpose({
	show,
	hide,
})
</script>
