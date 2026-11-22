<template>
	<button
		type="button"
		class="checkbox-button group bg-transparent border-none p-0 m-0 flex items-center text-left gap-3 checkbox-outer text-contrast focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[--color-focus-ring] focus-visible:ring-offset-2 focus-visible:ring-offset-[--surface-1] rounded-md select-none"
		:disabled="disabled"
		:class="
			disabled
				? 'cursor-not-allowed opacity-50'
				: 'cursor-pointer hover:brightness-[--hover-brightness]'
		"
		:aria-label="description || label || undefined"
		:aria-checked="indeterminate ? 'mixed' : modelValue"
		role="checkbox"
		@click="toggle"
	>
		<span
			class="checkbox-box relative w-5 h-5 aspect-square rounded-md flex shrink-0 items-center justify-center border border-solid overflow-hidden"
			:class="[
				modelValue || indeterminate
					? 'border-transparent text-white'
					: 'border-surface-5 text-primary group-hover:border-surface-5/80',
				{ 'checkbox-box--checked': modelValue || indeterminate },
			]"
		>
			<!-- Base background layer -->
			<span class="absolute inset-0 bg-surface-2 pointer-events-none rounded-[5px]" />

			<!-- Active gradient overlay for cubic-bezier color interpolation -->
			<span
				class="checkbox-gradient-overlay absolute inset-0 bg-gradient-to-r from-[#5b5bf0] to-[#22d3ee] pointer-events-none rounded-[5px]"
				:class="{
					'opacity-100': modelValue || indeterminate,
					'opacity-0': !modelValue && !indeterminate,
				}"
			/>

			<!-- Icons with pop entrance animation -->
			<MinusIcon
				v-if="indeterminate"
				class="checkbox-icon relative z-10"
				aria-hidden="true"
				stroke-width="3"
			/>
			<CheckIcon
				v-else-if="modelValue"
				class="checkbox-icon relative z-10"
				aria-hidden="true"
				stroke-width="3"
			/>
		</span>
		<!-- aria-hidden is set so screenreaders only use the <button>'s aria-label -->
		<span v-if="label" :class="labelClass" aria-hidden="true">
			{{ label }}
		</span>
		<slot v-else />
	</button>
</template>

<script setup lang="ts">
import { CheckIcon, MinusIcon } from '@modrinth/assets'
import type { HTMLAttributes } from 'vue'

const emit = defineEmits<{
	'update:modelValue': [modelValue: boolean, event?: MouseEvent]
}>()

const props = withDefaults(
	defineProps<{
		label?: string
		labelClass?: HTMLAttributes['class']
		disabled?: boolean
		description?: string
		modelValue: boolean
		clickEvent?: () => void
		indeterminate?: boolean
	}>(),
	{
		label: '',
		labelClass: '',
		disabled: false,
		description: '',
		modelValue: false,
		clickEvent: () => {},
		indeterminate: false,
	},
)

function toggle(event: MouseEvent) {
	if (!props.disabled) {
		emit('update:modelValue', !props.modelValue, event)
	}
}
</script>

<style scoped>
.checkbox-box {
	box-shadow: 1px 1px 2px 0 rgba(0, 0, 0, 0.08);
	transform: scale(1);
	transition:
		transform var(--duration-standard, 220ms)
			var(--ease-bounce, cubic-bezier(0.175, 0.885, 0.32, 1.275)),
		border-color var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		box-shadow var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.checkbox-box--checked {
	box-shadow:
		0 2px 8px -2px rgba(91, 91, 240, 0.5),
		0 1px 2px 0 rgba(0, 0, 0, 0.1);
}

.checkbox-button:not(:disabled):hover .checkbox-box:not(.checkbox-box--checked) {
	box-shadow: 0 2px 5px 0 rgba(0, 0, 0, 0.12);
}

.checkbox-button:not(:disabled):hover .checkbox-box--checked {
	box-shadow:
		0 3px 10px -2px rgba(91, 91, 240, 0.6),
		0 1px 3px 0 rgba(0, 0, 0, 0.15);
}

/* Tactile down-scale on press (120ms anticipation) */
.checkbox-button:not(:disabled):active .checkbox-box {
	transform: scale(0.92);
	box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.08);
	transition:
		transform var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		box-shadow var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

/* Color interpolation via opacity layer */
.checkbox-gradient-overlay {
	transition: opacity var(--duration-standard, 220ms)
		var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

/* Check icon pop entrance */
.checkbox-icon {
	animation: check-pop 180ms var(--ease-bounce, cubic-bezier(0.175, 0.885, 0.32, 1.275)) forwards;
}

@keyframes check-pop {
	0% {
		transform: scale(0.4);
		opacity: 0;
	}
	70% {
		transform: scale(1.15);
		opacity: 1;
	}
	100% {
		transform: scale(1);
		opacity: 1;
	}
}

@media (prefers-reduced-motion: reduce) {
	.checkbox-box,
	.checkbox-gradient-overlay,
	.checkbox-icon {
		animation: none !important;
		transition: none !important;
	}
	.checkbox-button:not(:disabled):active .checkbox-box {
		transform: none !important;
	}
}
</style>
