<template>
	<div class="flex flex-col w-full h-full p-6 max-w-7xl mx-auto gap-6 overflow-hidden">
		<!-- Page Header -->
		<div
			class="flex flex-wrap items-center justify-between gap-4 border-b border-solid border-surface-5 pb-4"
		>
			<div class="flex flex-col gap-1">
				<div class="flex items-center gap-2">
					<NewspaperIcon class="size-6 text-brand" />
					<h1 class="m-0 text-2xl font-bold text-contrast">
						{{ formatMessage(messages.title) }}
					</h1>
				</div>
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.subtitle) }}
				</p>
			</div>

			<div class="flex items-center gap-3">
				<!-- Channel Selector -->
				<div class="flex rounded-xl bg-surface-3 p-1 border border-solid border-surface-5">
					<button
						type="button"
						:class="[
							'rounded-lg px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer border-0',
							channel === 'upstream'
								? 'bg-brand text-white shadow-sm'
								: 'bg-transparent text-secondary hover:text-contrast',
						]"
						@click="changeChannel('upstream')"
					>
						{{ formatMessage(messages.channelEngine) }}
					</button>
					<button
						type="button"
						:class="[
							'rounded-lg px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer border-0',
							channel === 'offline'
								? 'bg-brand text-white shadow-sm'
								: 'bg-transparent text-secondary hover:text-contrast',
						]"
						@click="changeChannel('offline')"
					>
						{{ formatMessage(messages.channelOffline) }}
					</button>
				</div>

				<!-- Refresh Button -->
				<Button
					type="outlined"
					size="sm"
					:disabled="loading || !!installingTag || !!removingTag"
					@click="fetchReleases"
				>
					<RefreshCwIcon :class="{ 'animate-spin': loading }" class="size-4" />
					<span>{{ formatMessage(commonMessages.refreshButton) }}</span>
				</Button>
			</div>
		</div>

		<!-- Search and Filter Bar -->
		<div class="flex flex-wrap items-center justify-between gap-4">
			<div class="relative flex-1 min-w-[220px] max-w-md">
				<SearchIcon
					class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 size-4 text-secondary"
				/>
				<input
					v-model="searchQuery"
					type="text"
					:placeholder="formatMessage(messages.searchPlaceholder)"
					class="w-full rounded-xl border border-solid border-surface-5 bg-surface-2 py-2 pl-9 pr-3 text-xs text-contrast placeholder:text-secondary focus:border-brand focus:outline-none"
				/>
			</div>

			<!-- Filter Pills -->
			<div class="flex items-center gap-1.5">
				<button
					v-for="f in filterOptions"
					:key="f.key"
					type="button"
					:class="[
						'rounded-lg px-3 py-1 text-xs font-medium transition-colors cursor-pointer border border-solid',
						filter === f.key
							? 'border-brand/40 bg-brand/10 text-brand'
							: 'border-surface-5 bg-surface-2 text-secondary hover:text-contrast',
					]"
					@click="filter = f.key"
				>
					{{ f.label }}
				</button>
			</div>
		</div>

		<!-- Error Alert -->
		<div
			v-if="error"
			class="flex items-center justify-between rounded-xl bg-red-500/10 p-3 text-xs text-red-400"
		>
			<span>{{ error }}</span>
			<Button size="sm" type="quiet" @click="fetchReleases">
				{{ formatMessage(messages.retry) }}
			</Button>
		</div>

		<!-- Main Content Area: Split View -->
		<div v-if="loading" class="flex flex-1 items-center justify-center">
			<div class="flex flex-col items-center gap-2">
				<SpinnerIcon class="size-8 animate-spin text-brand" />
				<span class="text-xs text-secondary">{{ formatMessage(messages.loading) }}</span>
			</div>
		</div>

		<div
			v-else-if="filteredReleases.length === 0"
			class="flex flex-1 flex-col items-center justify-center rounded-2xl border border-solid border-surface-5 bg-surface-2 p-12 text-center text-secondary"
		>
			<NewspaperIcon class="size-10 text-secondary mb-2" />
			<p class="m-0 text-sm font-semibold text-contrast">
				{{ formatMessage(messages.noReleases) }}
			</p>
			<p class="m-0 mt-1 text-xs text-secondary">
				{{ formatMessage(messages.noReleasesSubtitle) }}
			</p>
		</div>

		<div v-else class="flex flex-1 gap-6 min-h-0 overflow-hidden">
			<!-- Left Column: Releases List -->
			<div
				class="flex flex-col w-80 shrink-0 gap-2 overflow-y-auto pr-2 border-r border-solid border-surface-5"
			>
				<button
					v-for="(rel, idx) in filteredReleases"
					:key="rel.tag"
					type="button"
					:class="[
						'flex flex-col text-left gap-1 rounded-xl border border-solid p-3 transition-colors cursor-pointer',
						selectedTag === rel.tag
							? 'border-brand bg-brand/10'
							: 'border-surface-5 bg-surface-2 hover:border-surface-6',
					]"
					@click="selectedTag = rel.tag"
				>
					<div class="flex items-center justify-between gap-1">
						<span class="font-mono text-xs font-bold text-contrast truncate">
							{{ rel.tag }}
						</span>
						<div class="flex items-center gap-1 shrink-0">
							<span
								v-if="idx === 0 && !rel.prerelease"
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
						</div>
					</div>

					<span v-if="rel.name && rel.name !== rel.tag" class="text-xs text-contrast line-clamp-1">
						{{ rel.name }}
					</span>

					<span class="text-[11px] text-secondary">
						{{ rel.published_at || 'Unknown date' }}
					</span>
				</button>
			</div>

			<!-- Right Column: Active Release Notes Details -->
			<div v-if="activeRelease" class="flex flex-1 flex-col gap-4 overflow-y-auto pl-2">
				<!-- Release Header Banner -->
				<div
					class="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-2 p-5"
				>
					<div class="flex flex-col gap-1.5 min-w-0">
						<div class="flex flex-wrap items-center gap-2">
							<h2 class="m-0 font-mono text-xl font-bold text-contrast">
								{{ activeRelease.tag }}
							</h2>
							<span
								v-if="activeRelease.prerelease"
								class="rounded bg-orange/20 px-2 py-0.5 text-xs font-semibold text-orange"
							>
								{{ formatMessage(messages.prerelease) }}
							</span>
							<span
								v-if="activeRelease.installed"
								class="flex items-center gap-1 rounded bg-green-500/20 px-2 py-0.5 text-xs font-semibold text-green-400"
							>
								<CheckIcon class="size-3.5" />
								{{ formatMessage(messages.installed) }}
							</span>
						</div>

						<p
							v-if="activeRelease.name && activeRelease.name !== activeRelease.tag"
							class="m-0 text-sm font-medium text-contrast"
						>
							{{ activeRelease.name }}
						</p>

						<div class="flex items-center gap-4 text-xs text-secondary">
							<span>{{ activeRelease.published_at }}</span>
							<span v-if="activeRelease.asset_size > 0">
								{{ (activeRelease.asset_size / (1024 * 1024)).toFixed(1) }} MB
							</span>
						</div>
					</div>

					<!-- Actions for Selected Release -->
					<div class="flex items-center gap-2">
						<!-- Install / Uninstall Button -->
						<template v-if="activeRelease.installed">
							<Button
								size="sm"
								type="quiet"
								color="red"
								:disabled="removingTag === activeRelease.tag"
								@click="handleUninstall(activeRelease.tag)"
							>
								<SpinnerIcon v-if="removingTag === activeRelease.tag" class="size-4 animate-spin" />
								<TrashIcon v-else class="size-4" />
								<span>{{ formatMessage(messages.uninstall) }}</span>
							</Button>
						</template>
						<template v-else-if="activeRelease.asset_url">
							<Button
								size="sm"
								type="colored"
								color="brand"
								:disabled="installingTag === activeRelease.tag"
								@click="handleInstall(activeRelease.tag)"
							>
								<SpinnerIcon
									v-if="installingTag === activeRelease.tag"
									class="size-4 animate-spin"
								/>
								<DownloadIcon v-else class="size-4" />
								<span>{{ formatMessage(messages.installBuild) }}</span>
							</Button>
						</template>

						<!-- GitHub Link -->
						<Button size="sm" type="outlined" @click="openGitHubRelease(activeRelease.tag)">
							<GlobeIcon class="size-4" />
							<span>{{ formatMessage(messages.viewOnGithub) }}</span>
							<ExternalIcon class="size-3.5 text-secondary" />
						</Button>
					</div>
				</div>

				<!-- Markdown Body Card -->
				<Card class="flex flex-col p-6">
					<ProjectPageDescription
						v-if="activeRelease.body && activeRelease.body.trim().length > 0"
						:description="activeRelease.body"
					/>
					<div v-else class="py-12 text-center text-secondary">
						<p class="m-0 text-sm">{{ formatMessage(messages.noNotes) }}</p>
					</div>
				</Card>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	CheckIcon,
	DownloadIcon,
	ExternalIcon,
	GlobeIcon,
	NewspaperIcon,
	RefreshCwIcon,
	SearchIcon,
	SpinnerIcon,
	TrashIcon,
} from '@modrinth/assets'
import {
	Button,
	Card,
	commonMessages,
	defineMessages,
	ProjectPageDescription,
	useVIntl,
} from '@modrinth/ui'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, onMounted, ref } from 'vue'

import {
	getHorizonReleases,
	type HorizonRelease,
	installHorizonVersion,
	removeHorizonVersion,
} from '@/helpers/horizon'

const { formatMessage } = useVIntl()

const channel = ref<'upstream' | 'offline'>('upstream')
const releases = ref<HorizonRelease[]>([])
const loading = ref(true)
const error = ref<string | null>(null)
const selectedTag = ref<string | null>(null)
const searchQuery = ref('')
const filter = ref<'all' | 'stable' | 'prerelease' | 'installed'>('all')

const installingTag = ref<string | null>(null)
const removingTag = ref<string | null>(null)

const messages = defineMessages({
	title: {
		id: 'patch-notes.page.title',
		defaultMessage: 'Gradle Patch Notes',
	},
	subtitle: {
		id: 'patch-notes.page.subtitle',
		defaultMessage: 'Changelogs, new features, and fixes across native Gradle runtime releases.',
	},
	channelEngine: {
		id: 'patch-notes.page.channel-engine',
		defaultMessage: 'Engine Changelog',
	},
	channelOffline: {
		id: 'patch-notes.page.channel-offline',
		defaultMessage: 'Offline Builds',
	},
	searchPlaceholder: {
		id: 'patch-notes.page.search-placeholder',
		defaultMessage: 'Filter releases or notes...',
	},
	filterAll: {
		id: 'patch-notes.page.filter-all',
		defaultMessage: 'All',
	},
	filterStable: {
		id: 'patch-notes.page.filter-stable',
		defaultMessage: 'Stable',
	},
	filterPrerelease: {
		id: 'patch-notes.page.filter-prerelease',
		defaultMessage: 'Pre-releases',
	},
	filterInstalled: {
		id: 'patch-notes.page.filter-installed',
		defaultMessage: 'Installed',
	},
	installBuild: {
		id: 'patch-notes.page.install-build',
		defaultMessage: 'Install Build',
	},
	uninstall: {
		id: 'patch-notes.page.uninstall',
		defaultMessage: 'Uninstall',
	},
	installed: {
		id: 'patch-notes.page.installed',
		defaultMessage: 'Installed',
	},
	latest: {
		id: 'patch-notes.page.latest',
		defaultMessage: 'Latest',
	},
	prerelease: {
		id: 'patch-notes.page.prerelease',
		defaultMessage: 'Pre-release',
	},
	viewOnGithub: {
		id: 'patch-notes.page.view-github',
		defaultMessage: 'View on GitHub',
	},
	noNotes: {
		id: 'patch-notes.page.no-notes',
		defaultMessage: 'No release notes provided for this version.',
	},
	loading: {
		id: 'patch-notes.page.loading',
		defaultMessage: 'Loading patch notes...',
	},
	retry: {
		id: 'patch-notes.page.retry',
		defaultMessage: 'Retry',
	},
	noReleases: {
		id: 'patch-notes.page.no-releases',
		defaultMessage: 'No releases found',
	},
	noReleasesSubtitle: {
		id: 'patch-notes.page.no-releases-subtitle',
		defaultMessage: 'Try adjusting your search query or filters.',
	},
})

const filterOptions = computed(() => [
	{ key: 'all' as const, label: formatMessage(messages.filterAll) },
	{ key: 'stable' as const, label: formatMessage(messages.filterStable) },
	{ key: 'prerelease' as const, label: formatMessage(messages.filterPrerelease) },
	{ key: 'installed' as const, label: formatMessage(messages.filterInstalled) },
])

const filteredReleases = computed(() => {
	let list = releases.value

	if (filter.value === 'stable') {
		list = list.filter((r) => !r.prerelease)
	} else if (filter.value === 'prerelease') {
		list = list.filter((r) => r.prerelease)
	} else if (filter.value === 'installed') {
		list = list.filter((r) => r.installed)
	}

	const q = searchQuery.value.trim().toLowerCase()
	if (q) {
		list = list.filter(
			(r) =>
				r.tag.toLowerCase().includes(q) ||
				r.name.toLowerCase().includes(q) ||
				r.body.toLowerCase().includes(q),
		)
	}

	return list
})

const activeRelease = computed(() => {
	if (!selectedTag.value && filteredReleases.value.length > 0) {
		return filteredReleases.value[0]
	}
	return (
		releases.value.find((r) => r.tag === selectedTag.value) ?? filteredReleases.value[0] ?? null
	)
})

async function fetchReleases() {
	loading.value = true
	error.value = null
	try {
		const repoTarget =
			channel.value === 'upstream' ? 'horizon-rust/horizon' : 'gerdaroot/Horizon-Offline-Launcher'
		const res = await getHorizonReleases(repoTarget)
		releases.value = res
		if (res.length > 0 && !selectedTag.value) {
			selectedTag.value = res[0].tag
		}
	} catch (e) {
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		loading.value = false
	}
}

function changeChannel(c: 'upstream' | 'offline') {
	if (channel.value === c) return
	channel.value = c
	selectedTag.value = null
	fetchReleases()
}

async function handleInstall(tag: string) {
	installingTag.value = tag
	try {
		await installHorizonVersion(tag)
		await fetchReleases()
	} catch (e) {
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		installingTag.value = null
	}
}

async function handleUninstall(tag: string) {
	removingTag.value = tag
	try {
		await removeHorizonVersion(tag)
		await fetchReleases()
	} catch (e) {
		error.value = e instanceof Error ? e.message : String(e)
	} finally {
		removingTag.value = null
	}
}

function openGitHubRelease(tag: string) {
	const repo =
		channel.value === 'upstream' ? 'horizon-rust/horizon' : 'gerdaroot/Horizon-Offline-Launcher'
	const url = `https://github.com/${repo}/releases/tag/${encodeURIComponent(tag)}`
	void openUrl(url)
}

onMounted(() => {
	fetchReleases()
})
</script>

<script lang="ts">
export default {
	name: 'PatchNotes',
}
</script>
