<script setup>
import {
	DownloadIcon,
	GameIcon,
	PlayIcon,
	SpinnerIcon,
	StopCircleIcon,
	TimerIcon,
} from '@modrinth/assets'
import {
	Avatar,
	defineMessages,
	IconButton,
	injectNotificationManager,
	useRelativeTime,
	useVIntl,
} from '@modrinth/ui'
import dayjs from 'dayjs'
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { useAppEvent } from '@/composables/use-app-event'
import { handleSevereError } from '@/composables/use-error.js'
import { trackEvent } from '@/helpers/analytics'
import { install_existing_instance, install_pack_to_existing_instance } from '@/helpers/install'
import { getInstanceIconUrl, kill, run } from '@/helpers/instance'
import { get_by_instance_id } from '@/helpers/process'
import { showInstanceInFolder } from '@/helpers/utils.js'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const formatRelativeTime = useRelativeTime()
const messages = defineMessages({
	instanceIcon: { id: 'app.instance.card.icon-alt', defaultMessage: 'Instance icon' },
	stop: { id: 'app.instance.card.stop', defaultMessage: 'Stop' },
	loading: { id: 'app.instance.card.loading', defaultMessage: 'Instance is loading...' },
	installing: { id: 'app.instance.card.installing', defaultMessage: 'Installing...' },
	play: { id: 'app.instance.card.play', defaultMessage: 'Play' },
	repair: { id: 'app.instance.card.repair', defaultMessage: 'Repair' },
	played: { id: 'app.instance.card.played', defaultMessage: 'Played {relativeTime}' },
	neverPlayed: { id: 'app.instance.card.never-played', defaultMessage: 'Never played' },
})

const props = defineProps({
	instance: {
		type: Object,
		default() {
			return {}
		},
	},
	compact: {
		type: Boolean,
		default: false,
	},
	first: {
		type: Boolean,
		default: false,
	},
})

const playing = ref(false)
const loading = ref(false)
const modLoading = computed(
	() =>
		loading.value ||
		currentEvent.value === 'installing' ||
		(currentEvent.value === 'launched' && !playing.value),
)
const installing = computed(() => props.instance.install_stage.includes('installing'))
const installed = computed(() => props.instance.install_stage === 'installed')

const isRustEdition = computed(() => {
	if (props.instance.runtime === 'java') return false
	if (props.instance.runtime === 'horizon') return true
	return props.instance.loader === 'horizon'
})

const badgeInfo = computed(() => {
	if (!isRustEdition.value) {
		return {
			label: 'Needs Java',
			class: 'bg-amber-500/15 text-amber-400 border border-amber-500/30',
		}
	}
	const loader = props.instance.loader?.toLowerCase()
	if (loader === 'vanilla' || loader === 'horizon' || !loader) {
		return {
			label: 'Native',
			class: 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30',
		}
	}
	return {
		label: 'Partially compatible',
		class: 'bg-sky-500/15 text-sky-400 border border-sky-500/30',
	}
})

const router = useRouter()

const seeInstance = async () => {
	await router.push(`/instance/${encodeURIComponent(props.instance.id)}`)
}

const checkProcess = async () => {
	const runningProcesses = await get_by_instance_id(props.instance.id).catch(handleError)

	playing.value = runningProcesses.length > 0
}

const play = async (e, context) => {
	e?.stopPropagation()
	if (props.instance.quarantined || installing.value || modLoading.value) return
	loading.value = true
	await run(props.instance.id)
		.catch((err) => handleSevereError(err, { instanceId: props.instance.id }))
		.finally(() => {
			trackEvent('InstanceStart', {
				loader: props.instance.loader,
				game_version: props.instance.game_version,
				source: context,
			})
		})
	loading.value = false
}

const stop = async (e, context) => {
	e?.stopPropagation()
	playing.value = false

	await kill(props.instance.id).catch(handleError)

	trackEvent('InstanceStop', {
		loader: props.instance.loader,
		game_version: props.instance.game_version,
		source: context,
	})
}

const repair = async (e) => {
	e?.stopPropagation()
	if (props.instance.quarantined || installing.value || modLoading.value) return

	if (
		props.instance.install_stage !== 'pack_installed' &&
		(props.instance.link?.type === 'modrinth_modpack' ||
			props.instance.link?.type === 'server_project_modpack')
	) {
		await install_pack_to_existing_instance(props.instance.id, {
			type: 'fromVersionId',
			project_id: props.instance.link.project_id ?? props.instance.link.server_project_id ?? '',
			version_id: props.instance.link.version_id ?? props.instance.link.content_version_id ?? '',
			title: props.instance.name,
		}).catch(handleError)
	} else {
		await install_existing_instance(props.instance.id, false).catch(handleError)
	}
}

const openFolder = async () => {
	await showInstanceInFolder(props.instance.id)
}

const addContent = async () => {
	if (props.instance.quarantined || installing.value || modLoading.value) return
	await router.push({
		path: `/browse/${props.instance.loader === 'vanilla' ? 'datapack' : 'mod'}`,
		query: { i: props.instance.id },
	})
}

defineExpose({
	play,
	stop,
	seeInstance,
	openFolder,
	addContent,
	instance: props.instance,
})

const currentEvent = ref(null)

useAppEvent('process', (e) => {
	if (e.instance_id === props.instance.id) {
		currentEvent.value = e.event
		if (e.event === 'finished') {
			playing.value = false
		}
	}
})

onMounted(() => {
	checkProcess()
})
</script>

<template>
	<template v-if="compact">
		<div
			class="instance-card instance-card-compact grid grid-cols-[auto_1fr_auto] bg-surface-3 border border-solid border-surface-5 rounded-xl p-3 pl-4 gap-2 cursor-pointer"
			@click="seeInstance"
			@mouseenter="checkProcess"
		>
			<Avatar
				size="48px"
				:src="getInstanceIconUrl(instance.icon_path)"
				:tint-by="instance.id"
				:alt="formatMessage(messages.instanceIcon)"
				pad-transparent-corners
			/>
			<div
				class="h-full flex items-center justify-between gap-2 font-bold text-contrast leading-normal min-w-0 pr-1"
			>
				<span class="line-clamp-2 truncate">{{ instance.name }}</span>
				<span
					:class="[
						'status-badge shrink-0 inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider border border-solid',
						badgeInfo.class,
					]"
				>
					{{ badgeInfo.label }}
				</span>
			</div>
			<div class="flex items-center">
				<IconButton
					v-if="playing"
					v-tooltip="formatMessage(messages.stop)"
					type="colored"
					color="red"
					:label="formatMessage(messages.stop)"
					class="instance-action-btn instance-stop-btn is-playing"
					@mouseenter="checkProcess"
					@click="(e) => stop(e, 'InstanceCard')"
				>
					<StopCircleIcon />
				</IconButton>
				<IconButton
					v-else-if="modLoading"
					v-tooltip="formatMessage(messages.loading)"
					:label="formatMessage(messages.loading)"
					class="instance-action-btn"
					disabled
				>
					<SpinnerIcon class="animate-spin" />
				</IconButton>
				<IconButton
					v-else-if="!instance.quarantined"
					v-tooltip="formatMessage(messages.play)"
					:type="first ? 'colored' : 'base'"
					:color="first ? 'brand' : undefined"
					:label="formatMessage(messages.play)"
					class="instance-action-btn instance-play-btn"
					@click="(e) => play(e, 'InstanceCard')"
					@mouseenter="checkProcess"
				>
					<!-- Translate for optical centering -->
					<PlayIcon class="translate-x-[1px]" />
				</IconButton>
			</div>
			<div class="flex items-center col-span-3 gap-1 text-secondary font-semibold">
				<TimerIcon />
				<span class="text-sm tabular-nums">
					<template v-if="instance.last_played">
						{{
							formatMessage(messages.played, {
								relativeTime: formatRelativeTime(dayjs(instance.last_played).toISOString()),
							})
						}}
					</template>
					<template v-else>{{ formatMessage(messages.neverPlayed) }}</template>
				</span>
			</div>
		</div>
	</template>
	<div v-else>
		<div
			class="instance-card instance-card-regular bg-surface-3 border border-solid border-surface-5 p-4 rounded-xl flex gap-3 group cursor-pointer"
			@click="seeInstance"
			@mouseenter="checkProcess"
		>
			<div class="relative flex items-center justify-center">
				<Avatar
					size="48px"
					:src="getInstanceIconUrl(instance.icon_path)"
					:tint-by="instance.id"
					:alt="formatMessage(messages.instanceIcon)"
					:class="`transition-all ${modLoading || installing ? `brightness-[0.25] scale-[0.85]` : `group-hover:brightness-75`}`"
					pad-transparent-corners
				/>
				<div class="absolute inset-0 flex items-center justify-center">
					<IconButton
						v-if="playing"
						v-tooltip="formatMessage(messages.stop)"
						type="colored"
						color="red"
						size="xl"
						:label="formatMessage(messages.stop)"
						:class="{ 'scale-100 opacity-100 is-playing': playing }"
						class="instance-action-btn instance-stop-btn transition-all scale-75 origin-center opacity-0 card-shadow"
						@click="(e) => stop(e, 'InstanceCard')"
						@mouseenter="checkProcess"
					>
						<StopCircleIcon />
					</IconButton>
					<SpinnerIcon
						v-else-if="modLoading || installing"
						v-tooltip="formatMessage(modLoading ? messages.loading : messages.installing)"
						class="animate-spin w-8 h-8 text-contrast"
						tabindex="-1"
					/>
					<IconButton
						v-else-if="!installed && !instance.quarantined"
						v-tooltip="formatMessage(messages.repair)"
						type="colored"
						color="brand"
						size="xl"
						:label="formatMessage(messages.repair)"
						class="instance-action-btn instance-play-btn transition-all scale-75 origin-center opacity-0 card-shadow"
						@click="(e) => repair(e)"
					>
						<DownloadIcon />
					</IconButton>
					<IconButton
						v-else-if="!instance.quarantined"
						v-tooltip="formatMessage(messages.play)"
						type="colored"
						color="brand"
						size="xl"
						:label="formatMessage(messages.play)"
						class="instance-action-btn instance-play-btn transition-all scale-75 origin-center opacity-0 card-shadow"
						@click="(e) => play(e, 'InstanceCard')"
						@mouseenter="checkProcess"
					>
						<PlayIcon class="translate-x-[2px]" />
					</IconButton>
				</div>
			</div>
			<div class="flex flex-col gap-1 grow min-w-0">
				<div class="flex items-center justify-between gap-2">
					<p class="m-0 text-md font-bold text-contrast leading-tight line-clamp-1">
						{{ instance.name }}
					</p>
					<span
						v-if="isRustEdition"
						class="status-badge shrink-0 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider bg-purple-500/15 text-purple-400 border border-purple-500/30"
					>
						Rust
					</span>
					<span
						v-else
						class="status-badge shrink-0 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider bg-amber-500/15 text-amber-400 border border-amber-500/30"
					>
						Java
					</span>
				</div>
				<div class="flex items-center col-span-3 gap-1 text-secondary font-semibold mt-auto">
					<GameIcon class="shrink-0" />
					<span class="text-sm capitalize tabular-nums">
						{{ instance.loader }} {{ instance.game_version }}
					</span>
				</div>
			</div>
		</div>
	</div>
</template>

<style lang="scss" scoped>
.instance-card {
	position: relative;
	box-shadow:
		0 1px 3px rgba(0, 0, 0, 0.12),
		0 1px 2px rgba(0, 0, 0, 0.08);
	transition:
		transform 220ms cubic-bezier(0.2, 0, 0, 1),
		box-shadow 220ms cubic-bezier(0.2, 0, 0, 1),
		border-color 220ms cubic-bezier(0.2, 0, 0, 1),
		background-color 220ms cubic-bezier(0.2, 0, 0, 1);
	will-change: transform, box-shadow;

	&:hover {
		transform: translateY(-3px);
		border-color: rgba(91, 91, 240, 0.45);
		box-shadow:
			0 12px 28px -4px rgba(0, 0, 0, 0.4),
			0 6px 14px -2px rgba(91, 91, 240, 0.16);
	}

	&:active {
		transform: translateY(-1px);
		box-shadow:
			0 4px 12px -2px rgba(0, 0, 0, 0.3),
			0 2px 6px -1px rgba(91, 91, 240, 0.1);
		transition-duration: 120ms;
	}
}

.instance-card-regular {
	.instance-action-btn:not(.is-playing) {
		transform: scale(0.75);
		opacity: 0;
	}

	&:hover,
	&:focus-within {
		.instance-action-btn:not(.is-playing) {
			transform: scale(1);
			opacity: 1;
		}
	}

	.instance-action-btn.is-playing {
		transform: scale(1);
		opacity: 1;
	}
}

.instance-action-btn {
	transition:
		transform 140ms cubic-bezier(0.2, 0, 0, 1),
		box-shadow 180ms cubic-bezier(0.2, 0, 0, 1),
		filter 180ms cubic-bezier(0.2, 0, 0, 1),
		opacity 200ms cubic-bezier(0.2, 0, 0, 1) !important;
	will-change: transform, box-shadow;

	&:hover {
		transform: scale(1.08) !important;
	}

	&:active {
		transform: scale(0.92) !important;
		transition-duration: 90ms !important;
	}
}

.instance-play-btn {
	&:hover {
		box-shadow: 0 0 14px rgba(91, 91, 240, 0.45);
	}

	&:active {
		box-shadow:
			0 0 20px rgba(91, 91, 240, 0.75),
			0 0 8px rgba(34, 211, 238, 0.5) !important;
		filter: brightness(1.12);
	}
}

.instance-stop-btn {
	&:hover {
		box-shadow: 0 0 14px rgba(239, 68, 68, 0.45);
	}

	&:active {
		box-shadow: 0 0 20px rgba(239, 68, 68, 0.75) !important;
		filter: brightness(1.12);
	}
}

.status-badge {
	transition:
		background-color 220ms cubic-bezier(0.2, 0, 0, 1),
		border-color 220ms cubic-bezier(0.2, 0, 0, 1),
		color 220ms cubic-bezier(0.2, 0, 0, 1),
		box-shadow 220ms cubic-bezier(0.2, 0, 0, 1),
		transform 220ms cubic-bezier(0.2, 0, 0, 1);
	will-change: background-color, border-color, color;
}

.instance-card:hover .status-badge {
	box-shadow: 0 2px 6px rgba(0, 0, 0, 0.18);
}

@media (prefers-reduced-motion: reduce) {
	.instance-card {
		transform: none !important;
		transition: none !important;
	}

	.instance-action-btn {
		transform: none !important;
		box-shadow: none !important;
		transition: none !important;
	}

	.status-badge {
		transition: none !important;
	}
}
</style>
