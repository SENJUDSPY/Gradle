<script setup lang="ts">
import { Avatar, truncatedTooltip } from '@modrinth/ui'
import { computed, ref } from 'vue'

import { useAppSettings } from '@/composables/use-app-settings.ts'
import { useImageThumbnail } from '@/composables/use-image-thumbnail'
import { getInstanceIconUrl } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

const props = withDefaults(
	defineProps<{
		instance: GameInstance
		selected?: boolean
	}>(),
	{
		selected: false,
	},
)

const localIcon = computed(() => {
	const path = props.instance.icon_path
	return path && !/^https?:/.test(path) && !path.toLowerCase().endsWith('.svg') ? path : undefined
})
const appSettings = useAppSettings()
const compactMode = computed(() => appSettings.compactInstanceCards)
const thumbnail = useImageThumbnail(
	localIcon,
	() => (compactMode.value ? 96 : 384),
	() => String(props.instance.modified),
)
const iconSrc = computed(() =>
	localIcon.value ? thumbnail.value : getInstanceIconUrl(props.instance.icon_path),
)

const nameRef = ref<HTMLElement | null>(null)
const versionRef = ref<HTMLElement | null>(null)

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
</script>

<template>
	<div
		class="library-instance-card relative flex w-full min-w-0 select-none overflow-clip border border-solid bg-surface-3 text-left"
		:class="{
			'flex-row items-center justify-start gap-2.5 rounded-xl p-2.5': compactMode,
			'flex-col items-start justify-end gap-3 rounded-[20px] p-3': !compactMode,
			'[border-color:color-mix(in_srgb,var(--color-text-primary)_40%,transparent)] brightness-110':
				selected,
			'border-surface-4': !selected,
		}"
	>
		<div
			class="relative flex shrink-0 items-center max-w-full overflow-clip"
			:class="compactMode ? 'size-10 rounded-lg' : 'aspect-square min-w-full rounded-2xl'"
		>
			<Avatar
				class="pointer-events-none outline-none"
				:class="compactMode ? '!rounded-lg' : '!rounded-2xl'"
				size="100%"
				:src="iconSrc"
				loading="lazy"
				:tint-by="instance.id"
				alt=""
				no-shadow
				pad-transparent-corners
			/>
			<slot name="loading" :compact="compactMode" />
			<div
				class="library-leading-slot absolute z-[1] flex items-center justify-center"
				:class="compactMode ? 'inset-0' : 'bottom-1.5 right-1.5 size-12'"
			>
				<slot name="leading" :compact="compactMode" />
			</div>
		</div>
		<div
			class="flex min-w-0 w-full flex-col items-start justify-center gap-1 px-0.5"
			:class="{ 'pr-10': compactMode }"
		>
			<p
				ref="nameRef"
				v-tooltip="truncatedTooltip(nameRef, instance.name)"
				class="m-0 w-full truncate text-base font-semibold leading-5 text-contrast"
			>
				{{ instance.name }}
			</p>
			<div class="flex w-full items-center justify-between gap-1.5">
				<p
					ref="versionRef"
					v-tooltip="truncatedTooltip(versionRef, `${instance.loader} ${instance.game_version}`)"
					class="m-0 truncate text-sm font-medium capitalize leading-[18px] text-primary"
				>
					{{ instance.loader }} {{ instance.game_version }}
				</p>
				<span
					class="status-badge inline-flex shrink-0 items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold tracking-wider uppercase border border-solid"
					:class="badgeInfo.class"
				>
					{{ badgeInfo.label }}
				</span>
			</div>
		</div>
		<slot name="overlay" :compact="compactMode" />
	</div>
</template>

<style lang="scss" scoped>
.library-instance-card {
	position: relative;
	box-shadow:
		0 1px 3px rgba(0, 0, 0, 0.12),
		0 1px 2px rgba(0, 0, 0, 0.08);
	transition:
		transform 220ms cubic-bezier(0.2, 0, 0, 1),
		box-shadow 220ms cubic-bezier(0.2, 0, 0, 1),
		border-color 220ms cubic-bezier(0.2, 0, 0, 1),
		background-color 220ms cubic-bezier(0.2, 0, 0, 1),
		filter 220ms cubic-bezier(0.2, 0, 0, 1);
	will-change: transform, box-shadow;

	&:hover {
		transform: translateY(-2px);
		border-color: rgba(255, 255, 255, 0.16);
		box-shadow:
			0 8px 24px -4px rgba(0, 0, 0, 0.35),
			0 2px 6px rgba(0, 0, 0, 0.2);
	}

	&:active {
		transform: translateY(0);
		box-shadow:
			0 2px 8px -2px rgba(0, 0, 0, 0.25),
			0 1px 3px rgba(0, 0, 0, 0.15);
		transition-duration: 120ms;
	}
}

.library-leading-slot {
	:deep(.button-frame),
	:deep(button) {
		transition:
			transform 140ms cubic-bezier(0.2, 0, 0, 1),
			box-shadow 180ms cubic-bezier(0.2, 0, 0, 1),
			filter 180ms cubic-bezier(0.2, 0, 0, 1),
			opacity 200ms cubic-bezier(0.2, 0, 0, 1) !important;
		will-change: transform, box-shadow;

		&:hover {
			transform: scale(1.05) !important;
			filter: brightness(1.08);
			box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
		}

		&:active {
			transform: scale(0.95) translateY(0.5px) !important;
			filter: brightness(0.96);
			box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3) inset !important;
			transition-duration: 90ms !important;
		}
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

.library-instance-card:hover .status-badge {
	box-shadow: 0 2px 6px rgba(0, 0, 0, 0.18);
}

@media (prefers-reduced-motion: reduce) {
	.library-instance-card {
		transform: none !important;
		transition: none !important;
	}

	.library-leading-slot {
		:deep(.button-frame),
		:deep(button) {
			transform: none !important;
			box-shadow: none !important;
			transition: none !important;
		}
	}

	.status-badge {
		transition: none !important;
	}
}
</style>
