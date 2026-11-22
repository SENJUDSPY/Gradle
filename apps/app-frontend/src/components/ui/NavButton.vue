<template>
	<component
		:is="isLink ? RouterLink : 'button'"
		:to="isLink ? to : undefined"
		v-bind="$attrs"
		:disabled="isLink ? undefined : disabled"
		:active-class="isLink && isSubpage ? '' : undefined"
		:class="{
			'router-link-active': isPrimary && isPrimary(route),
			'subpage-active': isSubpage && isSubpage(route),
			disabled: disabled,
		}"
		class="nav-button border-none text-secondary cursor-pointer w-11 h-11 rounded-xl flex items-center justify-center text-xl bg-transparent hover:text-contrast"
		@click="onClick"
	>
		<slot />
	</component>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { RouteLocationNormalizedLoaded } from 'vue-router'
import { RouterLink, useRoute } from 'vue-router'

const route = useRoute()

type RouteFunction = (route: RouteLocationNormalizedLoaded) => boolean

const props = withDefaults(
	defineProps<{
		to: (() => void) | string
		isPrimary?: RouteFunction
		isSubpage?: RouteFunction
		highlightOverride?: boolean
		disabled?: boolean
	}>(),
	{
		disabled: false,
	},
)

const isLink = computed(() => typeof props.to === 'string')

function onClick() {
	if (typeof props.to === 'function') {
		props.to()
	}
}

defineOptions({
	inheritAttrs: false,
})
</script>

<style lang="scss" scoped>
.nav-button {
	position: relative;
	outline: none !important;
	border: none !important;
	transform: translateZ(0);
	transition:
		transform var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		color var(--duration-quick, 150ms) ease,
		background-color var(--duration-quick, 150ms) ease;
	will-change: transform;

	&:focus,
	&:focus-visible {
		outline: none !important;
		box-shadow: none !important;
	}

	// Ambient depth & primary interaction
	&:hover:not(.disabled) {
		transform: translateY(-1px) scale(1.04);
		background-color: var(--brand-gradient-button, rgba(255, 255, 255, 0.08));
		color: var(--color-contrast, #ffffff);
	}

	&:active:not(.disabled) {
		transform: translateY(1px) scale(0.94);
		transition-duration: 80ms;
	}

	&.disabled {
		opacity: 0.4;
		cursor: not-allowed;
		pointer-events: none;
		transform: none !important;
	}

	// Secondary Layer: Glowing backplate on active
	&::before {
		content: '';
		position: absolute;
		inset: 0;
		background: var(
			--brand-gradient,
			linear-gradient(135deg, #5b5bf0 0%, #3b82f6 50%, #22d3ee 100%)
		);
		border-radius: var(--radius-md, 12px);
		opacity: 0;
		transform: scale(0.7);
		z-index: -1;
		transition:
			opacity var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
			transform var(--duration-standard, 220ms)
				var(--ease-bounce, cubic-bezier(0.175, 0.885, 0.32, 1.275)),
			box-shadow var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
	}

	&:hover:not(.router-link-active):not(.disabled)::before {
		opacity: 0.08;
		transform: scale(0.98);
	}

	// Secondary Layer: Fluid Active Indicator Pill on the left edge
	&::after {
		content: '';
		position: absolute;
		left: -4px;
		top: 50%;
		transform: translateY(-50%) scaleY(0) scaleX(0.7);
		width: 3px;
		height: 20px;
		border-radius: 9999px;
		background: var(--color-brand, #5b5bf0);
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
		opacity: 0;
		transition:
			transform var(--duration-standard, 240ms)
				var(--ease-bounce, cubic-bezier(0.175, 0.885, 0.32, 1.275)),
			opacity var(--duration-quick, 140ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
			height var(--duration-standard, 240ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
	}
}

// Icon micro-motion & directional feedback
.nav-button {
	svg,
	:deep(svg) {
		transition: transform var(--duration-quick, 150ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
		will-change: transform;
	}

	&:hover:not(.disabled) {
		svg,
		:deep(svg) {
			transform: scale(1.06) translateY(-0.5px);
		}
	}

	&:active:not(.disabled) {
		svg,
		:deep(svg) {
			transform: scale(0.94) translateY(0.5px);
		}
	}
}

.router-link-active,
.subpage-active {
	svg,
	:deep(svg) {
		filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.25));
	}
}

.router-link-active {
	color: #ffffff !important;

	&::before {
		transform: scale(1);
		opacity: 1;
		box-shadow:
			0 1px 3px rgba(0, 0, 0, 0.25),
			inset 0 1px 0 rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	&::after {
		transform: translateY(-50%) scaleY(1) scaleX(1);
		opacity: 1;
		height: 20px;
	}

	&:hover:not(.disabled) {
		transform: translateY(-0.5px);
		&::before {
			box-shadow:
				0 2px 6px rgba(0, 0, 0, 0.3),
				inset 0 1px 0 rgba(255, 255, 255, 0.14);
		}
	}

	&:active:not(.disabled) {
		transform: translateY(0.5px) scale(0.98);
	}
}

.subpage-active {
	color: var(--color-contrast, #ffffff) !important;
	background-color: var(--surface-4, rgba(255, 255, 255, 0.08));
	border-radius: var(--radius-md, 12px);

	&::after {
		transform: translateY(-50%) scaleY(0.7) scaleX(1);
		opacity: 0.9;
		height: 16px;
	}
}

@media (prefers-reduced-motion: reduce) {
	.nav-button,
	.nav-button::before,
	.nav-button::after,
	.nav-button svg,
	.nav-button :deep(svg) {
		transition: none !important;
		animation: none !important;
		transform: none !important;
	}

	.nav-button.router-link-active::after {
		transform: translateY(-50%) scaleY(1) !important;
		opacity: 1 !important;
	}

	.nav-button.subpage-active::after {
		transform: translateY(-50%) scaleY(0.7) !important;
		opacity: 1 !important;
	}
}
</style>
