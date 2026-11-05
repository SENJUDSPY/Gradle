<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="!isPreparing"
		max-width="600px"
		width="600px"
		:on-hide="handleHide"
	>
		<div class="flex w-full flex-col gap-5">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.subtitle) }}
			</p>

			<!-- Current status card -->
			<div
				:class="[
					'flex items-center gap-3 rounded-xl border border-solid p-3.5 transition-colors',
					isPrepared
						? 'border-green-500/30 bg-green-500/10 text-green-400'
						: 'border-surface-5 bg-surface-2 text-contrast',
				]"
			>
				<CheckCircleIcon v-if="isPrepared" class="size-6 shrink-0 text-green-500" />
				<PackageIcon v-else class="size-6 shrink-0 text-secondary" />
				<div class="flex flex-col gap-0.5">
					<span class="text-sm font-semibold">
						{{
							isPrepared
								? formatMessage(messages.statusPrepared)
								: formatMessage(messages.statusNotPrepared)
						}}
					</span>
					<span v-if="isPrepared && hasSounds" class="text-xs text-secondary">
						{{ formatMessage(messages.soundNotice) }}
					</span>
					<span v-else-if="isPrepared && !hasSounds" class="text-xs text-orange">
						{{ formatMessage(messages.soundWarning) }}
					</span>
				</div>
			</div>

			<!-- Input Jar Selection -->
			<div
				class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
			>
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<FileArchiveIcon class="size-5 text-secondary" />
						<span class="text-sm font-medium text-contrast">
							{{ formatMessage(messages.customJar) }}
						</span>
					</div>
					<Button size="sm" type="outlined" :disabled="isPreparing" @click="pickCustomJar">
						<FolderOpenIcon />
						{{ formatMessage(messages.chooseJar) }}
					</Button>
				</div>
				<div
					v-if="selectedJar"
					class="break-all rounded bg-surface-1 px-2.5 py-1.5 font-mono text-[11px] text-secondary"
				>
					{{ selectedJar }}
				</div>
				<div v-else class="text-xs text-secondary italic">
					{{ formatMessage(messages.autoLocating) }}
				</div>
			</div>

			<!-- Optional Minecraft Directory for Sounds -->
			<div
				class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
			>
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<FolderIcon class="size-5 text-secondary" />
						<span class="text-sm font-medium text-contrast">
							{{ formatMessage(messages.customMc) }}
						</span>
					</div>
					<Button size="sm" type="outlined" :disabled="isPreparing" @click="pickCustomMinecraftDir">
						<FolderOpenIcon />
						{{ formatMessage(messages.chooseDir) }}
					</Button>
				</div>
				<div
					v-if="selectedMcDir"
					class="break-all rounded bg-surface-1 px-2.5 py-1.5 font-mono text-[11px] text-secondary"
				>
					{{ selectedMcDir }}
				</div>
				<div v-else class="text-xs text-secondary italic">
					{{ formatMessage(messages.defaultMcDir) }}
				</div>
			</div>

			<!-- Active Progress or Error Display -->
			<div
				v-if="isPreparing || progress"
				class="flex flex-col gap-2.5 rounded-xl border border-solid border-surface-5 bg-surface-2 p-4"
			>
				<div class="flex items-center justify-between text-xs">
					<span class="font-semibold text-contrast">
						{{ progress?.stage || formatMessage(messages.preparingStatus) }}
					</span>
					<span class="font-mono text-secondary">
						{{ Math.round((progress?.percent ?? 0) * 100) }}%
					</span>
				</div>

				<ProgressBar
					:progress="progress?.percent ?? 0"
					:max="1"
					color="brand"
					:animated="isPreparing"
				/>

				<p class="m-0 text-xs text-secondary">
					{{ progress?.details || '' }}
				</p>

				<div v-if="error" class="mt-1 rounded-lg bg-red-500/10 p-2.5 text-xs text-red-400">
					{{ error }}
				</div>
			</div>

			<div
				v-if="success && !isPreparing"
				class="flex items-center gap-2 rounded-xl bg-green-500/10 p-3 text-sm font-medium text-green-400"
			>
				<CheckCircleIcon class="size-5 shrink-0" />
				<span>{{ formatMessage(messages.success) }}</span>
			</div>

			<!-- Footer Actions -->
			<div class="flex items-center justify-between border-t border-solid border-surface-5 pt-3">
				<Button v-if="!isPreparing" type="outlined" @click="checkStatus">
					<RefreshCwIcon />
					{{ formatMessage(messages.refresh) }}
				</Button>
				<div v-else />

				<div class="flex items-center gap-2">
					<Button v-if="!isPreparing" type="quiet" @click="hide">
						{{ formatMessage(commonMessages.cancelButton) }}
					</Button>
					<Button type="colored" color="brand" :disabled="isPreparing" @click="handleMainAction">
						<SpinnerIcon v-if="isPreparing" class="size-4 animate-spin" />
						<WrenchIcon v-else-if="!isPrepared" />
						<CheckIcon v-else />
						{{
							isPreparing
								? formatMessage(messages.buttonPreparing)
								: isPrepared
									? formatMessage(messages.buttonDone)
									: formatMessage(messages.buttonPrepare)
						}}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckCircleIcon,
	CheckIcon,
	FileArchiveIcon,
	FolderIcon,
	FolderOpenIcon,
	PackageIcon,
	RefreshCwIcon,
	SpinnerIcon,
	WrenchIcon,
} from '@modrinth/assets'
import {
	Button,
	commonMessages,
	defineMessages,
	NewModal,
	ProgressBar,
	useVIntl,
} from '@modrinth/ui'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { onUnmounted, ref } from 'vue'

import {
	checkHorizonRequirements,
	hasHorizonSounds,
	isHorizonResourcesPrepared,
	onHorizonPrepareProgress,
	type PreparationProgress,
	prepareHorizonResources,
} from '@/helpers/horizon'

const emit = defineEmits<{
	(e: 'prepared'): void
}>()

const messages = defineMessages({
	title: {
		id: 'app.horizon.preparation.title',
		defaultMessage: 'Prepare Horizon Game Files',
	},
	subtitle: {
		id: 'app.horizon.preparation.subtitle',
		defaultMessage:
			'Rebuild native Horizon game resources locally from your genuine Minecraft 26.3 copy. The launcher never downloads or redistributes Mojang game files.',
	},
	statusPrepared: {
		id: 'app.horizon.preparation.status.prepared',
		defaultMessage: 'Horizon resources are prepared and up to date.',
	},
	statusNotPrepared: {
		id: 'app.horizon.preparation.status.not-prepared',
		defaultMessage:
			'Horizon game files are not yet prepared. Preparation is required to launch the game.',
	},
	soundNotice: {
		id: 'app.horizon.preparation.sound-notice',
		defaultMessage: 'Audio sounds extracted from local asset store.',
	},
	soundWarning: {
		id: 'app.horizon.preparation.sound-warning',
		defaultMessage:
			'Minecraft sound index not found; sounds may be muted until assets are indexed.',
	},
	customJar: {
		id: 'app.horizon.preparation.custom-jar',
		defaultMessage: 'Custom 26.3 Client Jar',
	},
	chooseJar: {
		id: 'app.horizon.requirements.choose-jar',
		defaultMessage: 'Choose 26.3.jar...',
	},
	autoLocating: {
		id: 'app.horizon.preparation.auto-locating',
		defaultMessage: 'Auto-detecting genuine Minecraft 26.3 client jar...',
	},
	customMc: {
		id: 'app.horizon.preparation.custom-mc',
		defaultMessage: 'Minecraft Directory (Sounds)',
	},
	chooseDir: {
		id: 'app.horizon.preparation.choose-dir',
		defaultMessage: 'Select Directory...',
	},
	defaultMcDir: {
		id: 'app.horizon.preparation.default-dir',
		defaultMessage: 'Default system Minecraft directory will be scanned for audio files',
	},
	preparingStatus: {
		id: 'app.horizon.preparation.status.preparing',
		defaultMessage: 'Preparing Horizon game resources...',
	},
	buttonPrepare: {
		id: 'app.horizon.preparation.button.prepare',
		defaultMessage: 'Prepare Resources',
	},
	buttonPreparing: {
		id: 'app.horizon.preparation.button.preparing',
		defaultMessage: 'Preparing...',
	},
	buttonDone: {
		id: 'app.horizon.preparation.button.done',
		defaultMessage: 'Ready to Play',
	},
	success: {
		id: 'app.horizon.preparation.success',
		defaultMessage: 'Horizon resources prepared successfully!',
	},
	refresh: {
		id: 'app.horizon.requirements.check',
		defaultMessage: 'Check Requirements',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal>>()
const isPrepared = ref(false)
const hasSounds = ref(false)
const selectedJar = ref<string | null>(null)
const selectedMcDir = ref<string | null>(null)
const isPreparing = ref(false)
const progress = ref<PreparationProgress | null>(null)
const error = ref<string | null>(null)
const success = ref(false)
let onCompleteCallback: (() => void) | null = null
let unlistenProgress: UnlistenFn | null = null

async function checkStatus() {
	try {
		isPrepared.value = await isHorizonResourcesPrepared()
		if (isPrepared.value) {
			hasSounds.value = await hasHorizonSounds()
		}
		if (!selectedJar.value) {
			const report = await checkHorizonRequirements(null)
			if (report.minecraft_26_3.path) {
				selectedJar.value = report.minecraft_26_3.path
			}
		}
	} catch (e) {
		console.error('Failed to check Horizon status:', e)
	}
}

async function pickCustomJar() {
	const file = await open({
		multiple: false,
		filters: [{ name: 'Minecraft Client Jar', extensions: ['jar'] }],
	})
	if (typeof file === 'string') {
		selectedJar.value = file
		await checkStatus()
	}
}

async function pickCustomMinecraftDir() {
	const dir = await open({
		directory: true,
		multiple: false,
	})
	if (typeof dir === 'string') {
		selectedMcDir.value = dir
	}
}

async function startPreparation() {
	if (isPreparing.value) return
	isPreparing.value = true
	error.value = null
	success.value = false
	progress.value = {
		stage: 'Starting',
		details: 'Initializing resource preparation engine...',
		percent: 0.05,
		finished: false,
		error: null,
	}

	if (!unlistenProgress) {
		try {
			unlistenProgress = await onHorizonPrepareProgress((p) => {
				progress.value = p
				if (p.error) {
					error.value = p.error
				}
			})
		} catch (e) {
			console.error('Failed to listen to prepare progress:', e)
		}
	}

	try {
		await prepareHorizonResources(selectedJar.value, selectedMcDir.value)
		isPrepared.value = true
		hasSounds.value = await hasHorizonSounds()
		success.value = true
		emit('prepared')
		if (onCompleteCallback) {
			const cb = onCompleteCallback
			onCompleteCallback = null
			hide()
			cb()
		}
	} catch (e) {
		const msg = e instanceof Error ? e.message : String(e)
		error.value = msg
	} finally {
		isPreparing.value = false
	}
}

function handleMainAction() {
	if (isPrepared.value && !error.value) {
		if (onCompleteCallback) {
			const cb = onCompleteCallback
			onCompleteCallback = null
			hide()
			cb()
		} else {
			hide()
		}
	} else {
		void startPreparation()
	}
}

function show(onComplete?: () => void) {
	onCompleteCallback = onComplete ?? null
	error.value = null
	success.value = false
	modalRef.value?.show()
	void checkStatus()
}

function hide() {
	if (isPreparing.value) return
	modalRef.value?.hide()
}

function handleHide() {
	if (unlistenProgress) {
		unlistenProgress()
		unlistenProgress = null
	}
}

onUnmounted(() => {
	if (unlistenProgress) {
		unlistenProgress()
		unlistenProgress = null
	}
})

defineExpose({
	show,
	hide,
	checkStatus,
	isPrepared,
	hasSounds,
})
</script>
