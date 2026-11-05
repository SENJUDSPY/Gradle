<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="!installingTag"
		max-width="640px"
		width="640px"
	>
		<div class="flex w-full flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.subtitle) }}
			</p>

			<!-- Prompt banner if launched without any installed version -->
			<div
				v-if="installedVersions.length === 0 && !loading"
				class="flex items-center gap-2 rounded-xl bg-orange/10 p-3 text-sm font-medium text-orange"
			>
				<SparklesIcon class="size-5 shrink-0" />
				<span>{{ formatMessage(messages.requiredPrompt) }}</span>
			</div>

			<!-- Error display -->
			<div v-if="error" class="rounded-xl bg-red-500/10 p-3 text-xs text-red-400">
				{{ error }}
			</div>

			<!-- Auto (Latest Installed) option -->
			<div
				:class="[
					'flex items-center justify-between rounded-xl border border-solid p-3.5 transition-colors',
					!currentLoaderVersion ? 'border-brand/40 bg-brand/10' : 'border-surface-5 bg-surface-2',
				]"
			>
				<div class="flex flex-col gap-0.5">
					<div class="flex items-center gap-2">
						<span class="text-sm font-semibold text-contrast">
							{{ formatMessage(messages.auto) }}
						</span>
						<span
							v-if="!currentLoaderVersion"
							class="rounded bg-green-500/20 px-2 py-0.5 text-[11px] font-medium text-green-400"
						>
							{{ formatMessage(messages.active) }}
						</span>
					</div>
					<span class="text-xs text-secondary">
						{{
							installedVersions.length > 0
								? `Uses ${installedVersions[0]} (newest installed build)`
								: 'Automatically uses the newest installed Horizon release'
						}}
					</span>
				</div>
				<Button
					v-if="currentLoaderVersion"
					size="sm"
					type="outlined"
					:disabled="!!installingTag"
					@click="selectVersion(null)"
				>
					<CheckIcon />
					{{ formatMessage(messages.use) }}
				</Button>
			</div>

			<!-- Loading state -->
			<div v-if="loading" class="flex items-center justify-center py-10">
				<SpinnerIcon class="size-6 animate-spin text-brand" />
			</div>

			<!-- Releases list -->
			<div
				v-else-if="releases.length > 0"
				class="flex max-h-[380px] flex-col gap-2 overflow-y-auto pr-1"
			>
				<div
					v-for="(rel, idx) in releases"
					:key="rel.tag"
					:class="[
						'flex items-center justify-between rounded-xl border border-solid p-3 transition-colors',
						currentLoaderVersion === rel.tag
							? 'border-brand/40 bg-brand/10'
							: 'border-surface-5 bg-surface-2',
					]"
				>
					<div class="flex flex-col gap-1 min-w-0 pr-3">
						<div class="flex flex-wrap items-center gap-1.5">
							<span class="font-mono text-sm font-bold text-contrast">
								{{ rel.tag }}
							</span>
							<span
								v-if="idx === 0"
								class="rounded bg-brand/20 px-1.5 py-0.5 text-[10px] font-semibold text-brand"
							>
								{{ formatMessage(messages.latest) }}
							</span>
							<span
								v-if="rel.prerelease"
								class="rounded bg-orange/20 px-1.5 py-0.5 text-[10px] font-semibold text-orange"
							>
								{{ formatMessage(messages.prerelease) }}
							</span>
							<span
								v-if="rel.installed"
								class="rounded bg-blue-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-blue-400"
							>
								{{ formatMessage(messages.installed) }}
							</span>
							<span
								v-if="currentLoaderVersion === rel.tag"
								class="rounded bg-green-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-green-400"
							>
								{{ formatMessage(messages.active) }}
							</span>
						</div>

						<div class="flex items-center gap-3 text-xs text-secondary">
							<span>{{ rel.name !== rel.tag ? rel.name : '' }}</span>
							<span v-if="rel.published_at">{{ rel.published_at }}</span>
							<span v-if="rel.asset_size > 0">
								{{ (rel.asset_size / (1024 * 1024)).toFixed(1) }} MB
							</span>
						</div>
					</div>

					<!-- Actions for this release -->
					<div class="flex shrink-0 items-center gap-2">
						<!-- If installed -->
						<template v-if="rel.installed">
							<Button
								v-if="currentLoaderVersion !== rel.tag"
								size="sm"
								type="outlined"
								:disabled="!!installingTag"
								@click="selectVersion(rel.tag)"
							>
								<CheckIcon />
								{{ formatMessage(messages.use) }}
							</Button>
							<Button
								v-if="currentLoaderVersion !== rel.tag"
								size="sm"
								type="quiet"
								:disabled="!!installingTag || removingTag === rel.tag"
								@click="uninstallVersion(rel.tag)"
							>
								<SpinnerIcon v-if="removingTag === rel.tag" class="size-4 animate-spin" />
								<TrashIcon v-else class="size-4 text-red-400" />
							</Button>
						</template>

						<!-- If not installed -->
						<template v-else>
							<Button
								size="sm"
								type="colored"
								color="brand"
								:disabled="!!installingTag || !rel.asset_url"
								@click="handleInstall(rel.tag)"
							>
								<SpinnerIcon v-if="installingTag === rel.tag" class="size-4 animate-spin" />
								<DownloadIcon v-else />
								{{
									installingTag === rel.tag
										? formatMessage(messages.installing)
										: formatMessage(messages.install)
								}}
							</Button>
						</template>
					</div>
				</div>
			</div>

			<!-- Empty State -->
			<div v-else class="flex flex-col items-center justify-center py-8 text-center text-secondary">
				<p class="m-0 text-sm">{{ formatMessage(messages.noVersions) }}</p>
			</div>

			<!-- Footer -->
			<div class="flex items-center justify-between border-t border-solid border-surface-5 pt-3">
				<div class="flex items-center gap-2">
					<Button type="outlined" :disabled="loading || !!installingTag" @click="loadReleases">
						<RefreshCwIcon :class="{ 'animate-spin': loading }" />
						{{ formatMessage(commonMessages.refreshButton) }}
					</Button>
					<Button type="outlined" @click="openPatchNotes">
						<NewspaperIcon class="size-4" />
						{{ formatMessage(messages.viewPatchNotes) }}
					</Button>
				</div>

				<Button type="quiet" :disabled="!!installingTag" @click="hide">
					{{ formatMessage(commonMessages.closeButton) }}
				</Button>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckIcon,
	DownloadIcon,
	NewspaperIcon,
	RefreshCwIcon,
	SparklesIcon,
	SpinnerIcon,
	TrashIcon,
} from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { useQueryClient } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import {
	getHorizonReleases,
	getInstalledHorizonVersions,
	type HorizonRelease,
	installHorizonVersion,
	removeHorizonVersion,
} from '@/helpers/horizon'
import { edit } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { instanceKeys } from '@/pages/instance/query-options'

const props = defineProps<{
	instance?: GameInstance | null
}>()

const emit = defineEmits<{
	(e: 'selected', tag: string | null): void
	(e: 'installed', tag: string): void
}>()

const messages = defineMessages({
	title: {
		id: 'app.horizon.version.title',
		defaultMessage: 'Horizon Versions',
	},
	subtitle: {
		id: 'app.horizon.version.subtitle',
		defaultMessage:
			'Select, install, or update Horizon native client and server builds from the release feed.',
	},
	installed: {
		id: 'app.horizon.version.installed',
		defaultMessage: 'Installed',
	},
	active: {
		id: 'app.horizon.version.active',
		defaultMessage: 'Active',
	},
	install: {
		id: 'app.horizon.version.install',
		defaultMessage: 'Install',
	},
	installing: {
		id: 'app.horizon.version.installing',
		defaultMessage: 'Installing...',
	},
	use: {
		id: 'app.horizon.version.use',
		defaultMessage: 'Use this version',
	},
	uninstall: {
		id: 'app.horizon.version.uninstall',
		defaultMessage: 'Uninstall',
	},
	noVersions: {
		id: 'app.horizon.version.no-versions',
		defaultMessage: 'No Horizon releases found',
	},
	latest: {
		id: 'app.horizon.version.latest',
		defaultMessage: 'Latest',
	},
	prerelease: {
		id: 'app.horizon.version.prerelease',
		defaultMessage: 'Pre-release',
	},
	auto: {
		id: 'app.horizon.version.auto',
		defaultMessage: 'Auto (Latest Installed)',
	},
	requiredPrompt: {
		id: 'app.horizon.version.required-prompt',
		defaultMessage: 'Please install a Horizon build before launching this instance.',
	},
	viewPatchNotes: {
		id: 'app.horizon.version.view-patch-notes',
		defaultMessage: 'Patch Notes',
	},
})

const { formatMessage } = useVIntl()
const queryClient = useQueryClient()
const router = useRouter()
const modalRef = ref<InstanceType<typeof NewModal>>()

const releases = ref<HorizonRelease[]>([])
const installedVersions = ref<string[]>([])
const loading = ref(false)
const installingTag = ref<string | null>(null)
const removingTag = ref<string | null>(null)
const error = ref<string | null>(null)
let onInstalledLaunch: (() => void) | null = null

const currentLoaderVersion = computed(() => props.instance?.loader_version ?? null)

async function loadReleases() {
	loading.value = true
	error.value = null
	try {
		const [relList, instList] = await Promise.all([
			getHorizonReleases(),
			getInstalledHorizonVersions(),
		])
		releases.value = relList
		installedVersions.value = instList
	} catch (e) {
		console.error('Failed to load Horizon releases:', e)
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		loading.value = false
	}
}

async function handleInstall(tag: string) {
	installingTag.value = tag
	error.value = null
	try {
		await installHorizonVersion(tag)
		emit('installed', tag)

		// Refresh lists
		await loadReleases()

		// If this is the only installed version or user had a pending launch, select it
		if (props.instance?.id) {
			await selectVersion(tag)
		}

		if (onInstalledLaunch) {
			const cb = onInstalledLaunch
			onInstalledLaunch = null
			hide()
			cb()
		}
	} catch (e) {
		console.error('Failed to install Horizon release:', e)
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		installingTag.value = null
	}
}

async function selectVersion(tag: string | null) {
	if (!props.instance?.id) return
	try {
		await edit(props.instance.id, { loader_version: tag })
		await queryClient.invalidateQueries({ queryKey: instanceKeys.detail(props.instance.id) })
		await queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
		emit('selected', tag)
	} catch (e) {
		console.error('Failed to update instance Horizon version:', e)
		error.value = e instanceof Error ? e.message : String(e)
	}
}

async function uninstallVersion(tag: string) {
	removingTag.value = tag
	error.value = null
	try {
		await removeHorizonVersion(tag)
		await loadReleases()
	} catch (e) {
		console.error('Failed to remove Horizon version:', e)
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		removingTag.value = null
	}
}

function show(onCompleteLaunch?: () => void) {
	onInstalledLaunch = onCompleteLaunch ?? null
	error.value = null
	modalRef.value?.show()
	void loadReleases()
}

function hide() {
	if (installingTag.value) return
	modalRef.value?.hide()
}

function openPatchNotes() {
	hide()
	void router.push('/patch-notes')
}

defineExpose({
	show,
	hide,
	loadReleases,
	installedVersions,
})
</script>
