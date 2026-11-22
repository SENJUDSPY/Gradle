<template>
	<button
		:id="id"
		type="button"
		role="switch"
		:aria-checked="modelValue"
		:disabled="disabled"
		class="toggle-button group relative inline-flex shrink-0 touch-manipulation items-center rounded-full m-0 p-1 cursor-pointer select-none border border-solid overflow-hidden"
		:class="[
			small ? 'h-5 !w-[40px]' : 'h-6 !w-[48px]',
			modelValue ? 'border-transparent' : 'border-surface-5 hover:border-surface-5/80',
			disabled ? 'opacity-50 cursor-not-allowed' : '',
		]"
		@click="toggle"
	>
		<!-- Inactive background layer -->
		<span class="absolute inset-0 bg-surface-4 pointer-events-none rounded-full" />

		<!-- Active gradient overlay for smooth cubic-bezier color interpolation -->
		<span
			class="toggle-gradient-overlay absolute inset-0 bg-gradient-to-r from-[#5b5bf0] to-[#22d3ee] pointer-events-none rounded-full"
			:class="{
				'opacity-100 shadow-[0_0_12px_-2px_rgba(91,91,240,0.5)]': modelValue,
				'opacity-0': !modelValue,
			}"
		/>

		<!-- Thumb with spring-like travel & squash micro-interaction -->
		<span
			class="toggle-thumb relative z-10 rounded-full"
			:class="[
				small ? 'w-3 h-3' : 'w-4 h-4',
				modelValue
					? 'bg-white shadow-[0_1px_3px_rgba(0,0,0,0.3)]'
					: 'bg-secondary shadow-[0_1px_2px_rgba(0,0,0,0.15)]',
				thumbClass,
			]"
		/>
	</button>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'

const props = defineProps<{
	id?: string
	disabled?: boolean
	small?: boolean
}>()

const modelValue = defineModel<boolean>()

const isAnimating = ref(false)
const hasMounted = ref(false)
let animTimer: ReturnType<typeof setTimeout> | undefined

onMounted(() => {
	requestAnimationFrame(() => {
		hasMounted.value = true
	})
})

watch(
	() => modelValue.value,
	() => {
		if (!hasMounted.value) return
		isAnimating.value = true
		clearTimeout(animTimer)
		animTimer = setTimeout(() => {
			isAnimating.value = false
		}, 250)
	},
)

const thumbClass = computed(() => {
	if (isAnimating.value) {
		if (modelValue.value) {
			return props.small ? 'toggle-thumb--anim-on-small' : 'toggle-thumb--anim-on'
		} else {
			return props.small ? 'toggle-thumb--anim-off-small' : 'toggle-thumb--anim-off'
		}
	}

	return modelValue.value
		? props.small
			? 'translate-x-[20px]'
			: 'translate-x-[24px]'
		: 'translate-x-0'
})

function toggle() {
	if (!props.disabled) {
		modelValue.value = !modelValue.value
	}
}
</script>

<style scoped>
.toggle-button {
	transition:
		border-color var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		filter var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		transform var(--duration-standard, 220ms)
			var(--ease-bounce, cubic-bezier(0.175, 0.885, 0.32, 1.275));
}

.toggle-button:not(:disabled):hover {
	filter: brightness(var(--hover-brightness, 1.05));
}

.toggle-button:not(:disabled):active {
	transform: scale(0.97);
	transition: transform var(--duration-quick, 120ms)
		var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.toggle-button:focus-visible {
	outline: none;
	ring: 2px solid var(--color-focus-ring);
	box-shadow:
		0 0 0 2px var(--surface-1),
		0 0 0 4px var(--color-focus-ring);
}

.toggle-gradient-overlay {
	transition:
		opacity var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		box-shadow var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.toggle-thumb {
	will-change: transform;
	transform-origin: center center;
	transition:
		transform var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		background-color var(--duration-standard, 220ms)
			var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		box-shadow var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.toggle-button:not(:disabled):active .toggle-thumb:not([class*='toggle-thumb--anim']) {
	transform: scaleX(1.15);
}

/* Keyframe animations for spring-like travel with squash (scaleX: 1.15) */
@keyframes toggle-travel-on {
	0% {
		transform: translateX(0) scaleX(1) scaleY(1);
	}
	35% {
		transform: translateX(11px) scaleX(1.15) scaleY(0.92);
	}
	75% {
		transform: translateX(25px) scaleX(0.96) scaleY(1.04);
	}
	100% {
		transform: translateX(24px) scaleX(1) scaleY(1);
	}
}

@keyframes toggle-travel-off {
	0% {
		transform: translateX(24px) scaleX(1) scaleY(1);
	}
	35% {
		transform: translateX(13px) scaleX(1.15) scaleY(0.92);
	}
	75% {
		transform: translateX(-1px) scaleX(0.96) scaleY(1.04);
	}
	100% {
		transform: translateX(0) scaleX(1) scaleY(1);
	}
}

@keyframes toggle-travel-on-small {
	0% {
		transform: translateX(0) scaleX(1) scaleY(1);
	}
	35% {
		transform: translateX(9px) scaleX(1.15) scaleY(0.92);
	}
	75% {
		transform: translateX(21px) scaleX(0.96) scaleY(1.04);
	}
	100% {
		transform: translateX(20px) scaleX(1) scaleY(1);
	}
}

@keyframes toggle-travel-off-small {
	0% {
		transform: translateX(20px) scaleX(1) scaleY(1);
	}
	35% {
		transform: translateX(11px) scaleX(1.15) scaleY(0.92);
	}
	75% {
		transform: translateX(-1px) scaleX(0.96) scaleY(1.04);
	}
	100% {
		transform: translateX(0) scaleX(1) scaleY(1);
	}
}

.toggle-thumb--anim-on {
	animation: toggle-travel-on 240ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) forwards;
}

.toggle-thumb--anim-off {
	animation: toggle-travel-off 240ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) forwards;
}

.toggle-thumb--anim-on-small {
	animation: toggle-travel-on-small 240ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) forwards;
}

.toggle-thumb--anim-off-small {
	animation: toggle-travel-off-small 240ms var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)) forwards;
}

@media (prefers-reduced-motion: reduce) {
	.toggle-button,
	.toggle-thumb,
	.toggle-gradient-overlay {
		animation: none !important;
		transition: none !important;
	}
	.toggle-button:not(:disabled):active {
		transform: none !important;
	}
	.toggle-button:not(:disabled):active .toggle-thumb {
		transform: none !important;
	}
}
</style>
