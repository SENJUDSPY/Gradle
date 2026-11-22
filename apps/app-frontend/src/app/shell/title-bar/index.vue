<script setup lang="ts">
import { ChevronLeftIcon, ChevronRightIcon, RightArrowIcon } from '@modrinth/assets'
import { defineMessages, IconButton, useVIntl } from '@modrinth/ui'
import { useRouter } from 'vue-router'

import AppActionBar from './action-bar.vue'
import Breadcrumbs from './breadcrumbs.vue'
import WindowControls from './window-controls.vue'

defineProps<{
	canNavigateBack: boolean
	canNavigateForward: boolean
	showSidebarToggle: boolean
	sidebarToggled: boolean
}>()

const emit = defineEmits<{
	'toggle-sidebar': []
}>()

const router = useRouter()
const { formatMessage } = useVIntl()
const messages = defineMessages({
	goBack: { id: 'app.navigation.go-back', defaultMessage: 'Go back' },
	goForward: { id: 'app.navigation.go-forward', defaultMessage: 'Go forward' },
	nextImage: { id: 'app.navigation.next-image', defaultMessage: 'Next image' },
})
</script>

<template>
	<div
		data-tauri-drag-region
		class="app-title-bar relative z-[2] flex bg-bg-raised pl-1 [grid-area:status] [.mac-traffic-lights_&]:pl-20"
	>
		<div data-tauri-drag-region class="flex min-w-0 flex-1 items-center overflow-hidden p-2">
			<div data-tauri-drag-region class="ml-2 flex shrink-0 items-center gap-2">
				<IconButton
					type="outlined"
					:label="formatMessage(messages.goBack)"
					class="nav-history-button nav-back-button !h-7 !min-w-7 !w-7 !border !border-surface-4 !p-0 !opacity-100"
					:disabled="!canNavigateBack"
					@click="router.back()"
				>
					<ChevronLeftIcon
						class="nav-history-icon !size-4 !text-primary"
						:class="{ 'opacity-20': !canNavigateBack }"
					/>
				</IconButton>
				<IconButton
					type="outlined"
					:label="formatMessage(messages.goForward)"
					class="nav-history-button nav-forward-button !h-7 !min-w-7 !w-7 !border !border-surface-4 !p-0 !opacity-100"
					:disabled="!canNavigateForward"
					@click="router.forward()"
				>
					<ChevronRightIcon
						class="nav-history-icon !size-4 !text-primary"
						:class="{ 'opacity-20': !canNavigateForward }"
					/>
				</IconButton>
			</div>
			<Breadcrumbs />
		</div>
		<section data-tauri-drag-region class="flex shrink-0 ml-auto items-center">
			<IconButton
				v-if="showSidebarToggle"
				:type="sidebarToggled ? 'base' : 'quiet'"
				:label="formatMessage(messages.nextImage)"
				class="sidebar-toggle-btn mr-3"
				:class="{ 'rotate-180': !sidebarToggled }"
				@click="emit('toggle-sidebar')"
			>
				<RightArrowIcon class="sidebar-toggle-icon" />
			</IconButton>
			<div class="flex mr-3">
				<Suspense>
					<AppActionBar />
				</Suspense>
			</div>
			<WindowControls />
		</section>
	</div>
</template>

<style scoped lang="scss">
.app-title-bar {
	height: var(--top-bar-height);
	padding-right: var(--window-controls-width, 0px);
}

.nav-history-button {
	transition:
		transform var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		background-color var(--duration-quick, 120ms) ease,
		border-color var(--duration-quick, 120ms) ease;
	will-change: transform;

	&:hover:not(:disabled) {
		transform: scale(1.06);
	}

	&:active:not(:disabled) {
		transform: scale(0.92);
		transition-duration: 80ms;
	}

	.nav-history-icon {
		transition: transform var(--duration-quick, 140ms)
			var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
	}

	&.nav-back-button:hover:not(:disabled) .nav-history-icon {
		transform: translateX(-1.5px);
	}

	&.nav-forward-button:hover:not(:disabled) .nav-history-icon {
		transform: translateX(1.5px);
	}
}

.sidebar-toggle-btn {
	transition:
		transform var(--duration-slow, 280ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		background-color var(--duration-quick, 140ms) ease,
		box-shadow var(--duration-quick, 140ms) ease;
	will-change: transform;

	.sidebar-toggle-icon {
		transition: transform var(--duration-slow, 280ms)
			var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
	}

	&:hover:not(:disabled) {
		transform: scale(1.06);

		&.rotate-180 {
			transform: rotate(180deg) scale(1.06);
		}
	}

	&:active:not(:disabled) {
		transform: scale(0.94);
		transition-duration: 80ms;

		&.rotate-180 {
			transform: rotate(180deg) scale(0.94);
		}
	}

	&.rotate-180 {
		transform: rotate(180deg);
	}
}

@media (prefers-reduced-motion: reduce) {
	.nav-history-button,
	.nav-history-button .nav-history-icon,
	.sidebar-toggle-btn,
	.sidebar-toggle-btn .sidebar-toggle-icon {
		transition: none !important;
		transform: none !important;
	}

	.sidebar-toggle-btn.rotate-180 {
		transform: rotate(180deg) !important;
	}
}
</style>
