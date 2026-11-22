<script setup lang="ts">
import type { Component, CSSProperties } from 'vue'
import { computed, ref } from 'vue'

import type {
	ButtonColor,
	ButtonInteraction,
	ButtonNativeType,
	ButtonSize,
	ButtonType,
} from './types'

const baseClasses = [
	// Base
	'relative inline-flex min-w-0 shrink-0 items-center justify-center',
	'whitespace-nowrap border-0 no-underline',
	// Interactions
	'touch-manipulation cursor-pointer select-none',
	// Hovering
	'[&:not(:disabled):not([aria-disabled=true]):hover]:brightness-[--hover-brightness]',
	// Accessibility
	'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[--color-focus-ring] focus-visible:ring-offset-2 focus-visible:ring-offset-[--surface-1]',
	'disabled:cursor-not-allowed disabled:opacity-50',
	'[&[aria-disabled=true]]:cursor-not-allowed [&[aria-disabled=true]]:opacity-50',
].join(' ')

const sizeClasses: Record<ButtonSize, string> = {
	xs: 'h-9 min-h-[36px] gap-1.5 rounded-lg px-2.5 text-xs font-semibold leading-5 [&>svg]:size-4 [&>svg]:min-h-4 [&>svg]:min-w-4 [&>svg]:shrink-0',
	sm: 'h-9 min-h-[36px] gap-1.5 rounded-lg px-3 text-sm font-semibold leading-5 [&>svg]:size-4 [&>svg]:min-h-4 [&>svg]:min-w-4 [&>svg]:shrink-0',
	md: 'h-10 min-h-[40px] gap-2 rounded-lg px-4 text-sm font-semibold leading-5 [&>svg]:size-5 [&>svg]:min-h-5 [&>svg]:min-w-5 [&>svg]:shrink-0',
	lg: 'h-11 min-h-[44px] gap-2.5 rounded-lg px-5 text-base font-semibold leading-5 [&>svg]:size-5 [&>svg]:min-h-5 [&>svg]:min-w-5 [&>svg]:shrink-0',
	xl: 'h-12 min-h-[48px] gap-2.5 rounded-xl px-6 text-base font-bold leading-5 [&>svg]:size-6 [&>svg]:min-h-6 [&>svg]:min-w-6 [&>svg]:shrink-0',
}

const iconOnlySizeClasses: Record<ButtonSize, string> = {
	xs: 'w-9 min-w-[36px] !px-0',
	sm: 'w-9 min-w-[36px] !px-0',
	md: 'w-10 min-w-[40px] !px-0',
	lg: 'w-11 min-w-[44px] !px-0',
	xl: 'w-12 min-w-[48px] !px-0',
}

const typeClasses: Record<ButtonType, string> = {
	base: 'button-frame--base bg-surface-3 hover:bg-surface-4 text-contrast border border-solid border-surface-5 shadow-sm [&>svg]:text-primary',
	colored: 'button-frame--colored bg-[--button-color] text-white shadow-sm [&>svg]:text-inherit',
	'colored-text':
		'button-frame--colored-text bg-surface-3 hover:bg-surface-4 border border-solid border-surface-5 text-[--button-color] [&>svg]:text-inherit',
	outlined:
		'button-frame--outlined bg-transparent border border-solid border-surface-5 text-contrast hover:bg-surface-4/60 [&>svg]:text-inherit',
	quiet:
		'button-frame--quiet bg-transparent text-primary hover:text-contrast hover:bg-surface-4/60 [&>svg]:text-inherit',
}

const interactionClasses: Record<ButtonInteraction, string> = {
	surface:
		'[&:not(:disabled):not([aria-disabled=true]):hover]:bg-surface-4 [&:not(:disabled):not([aria-disabled=true]):focus-visible]:bg-surface-4',
	filled:
		'[&:not(:disabled):not([aria-disabled=true]):hover]:!bg-[--button-color] [&:not(:disabled):not([aria-disabled=true]):focus-visible]:!bg-[--button-color] [&:not(:disabled):not([aria-disabled=true]):hover]:!text-[var(--color-accent-contrast)] [&:not(:disabled):not([aria-disabled=true]):focus-visible]:!text-[var(--color-accent-contrast)]',
	none: '[&:not(:disabled):not([aria-disabled=true]):hover]:!brightness-100 [&:not(:disabled):not([aria-disabled=true]):focus-visible]:!brightness-100',
}

const colorVariables: Record<ButtonColor, string> = {
	brand: 'var(--color-brand)',
	red: 'var(--color-red)',
	orange: 'var(--color-orange)',
	green: 'var(--color-green)',
	blue: 'var(--color-blue)',
	purple: 'var(--color-purple)',
	medal_promotion: 'var(--medal-promotion-text-orange, var(--color-orange))',
}

const props = withDefaults(
	defineProps<{
		as: string | Component
		type?: ButtonType
		color?: ButtonColor
		size?: ButtonSize
		interaction?: ButtonInteraction
		iconOnly?: boolean
		circular?: boolean
		nativeType?: ButtonNativeType
	}>(),
	{
		type: 'base',
		size: 'md',
		interaction: 'surface',
		iconOnly: false,
		circular: false,
		nativeType: undefined,
	},
)

const element = ref<HTMLElement | null>(null)
const isBrand = computed(
	() => props.type === 'colored' && (props.color === 'brand' || !props.color),
)
const isDanger = computed(() => props.type === 'colored' && props.color === 'red')

const classes = computed(() => [
	baseClasses,
	typeClasses[props.type],
	isBrand.value ? 'button-frame--brand' : '',
	isDanger.value ? 'button-frame--danger' : '',
	props.type === 'quiet' ? interactionClasses[props.interaction] : '',
	sizeClasses[props.size],
	props.iconOnly ? iconOnlySizeClasses[props.size] : '',
	props.circular ? '!rounded-full' : '',
])
const style = computed((): CSSProperties | undefined => {
	if ((props.type === 'outlined' || props.type === 'quiet') && !props.color) return undefined
	if (
		props.type !== 'colored' &&
		props.type !== 'colored-text' &&
		props.type !== 'outlined' &&
		props.type !== 'quiet'
	)
		return undefined

	return {
		'--button-color': colorVariables[props.color ?? 'brand'],
	} as CSSProperties
})

defineExpose({ element })
</script>

<template>
	<component
		:is="as"
		ref="element"
		data-button
		:type="props.nativeType"
		:class="classes"
		:style="style"
	>
		<slot />
	</component>
</template>

<style scoped>
[data-button] {
	transform: scale(1);
	transition:
		transform var(--duration-standard, 220ms)
			var(--ease-bounce, cubic-bezier(0.175, 0.885, 0.32, 1.275)),
		box-shadow var(--duration-standard, 220ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		background-color var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		border-color var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		color var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		filter var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		opacity var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

[data-button]:not(:disabled):not([aria-disabled='true']):active {
	transform: scale(0.97);
	transition:
		transform var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1)),
		box-shadow var(--duration-quick, 120ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.button-frame--base,
.button-frame--colored-text {
	box-shadow: var(--shadow-sm, 0 1px 2px 0 rgba(0, 0, 0, 0.05));
}

.button-frame--base:not(:disabled):not([aria-disabled='true']):hover,
.button-frame--base:not(:disabled):not([aria-disabled='true']):focus-visible,
.button-frame--colored-text:not(:disabled):not([aria-disabled='true']):hover,
.button-frame--colored-text:not(:disabled):not([aria-disabled='true']):focus-visible {
	box-shadow:
		0 4px 10px -1px rgba(0, 0, 0, 0.12),
		0 2px 4px -2px rgba(0, 0, 0, 0.08);
}

.button-frame--base:not(:disabled):not([aria-disabled='true']):active,
.button-frame--colored-text:not(:disabled):not([aria-disabled='true']):active {
	box-shadow: 0 0 0 0 transparent;
}

.button-frame--colored {
	box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.25);
}

.button-frame--colored:not(:disabled):not([aria-disabled='true']):hover,
.button-frame--colored:not(:disabled):not([aria-disabled='true']):focus-visible {
	box-shadow:
		0 4px 12px 0 rgba(0, 0, 0, 0.3),
		0 0 14px -2px var(--button-color, rgba(0, 0, 0, 0.25));
}

.button-frame--colored:not(:disabled):not([aria-disabled='true']):active {
	box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.2);
}

.button-frame--brand {
	background: var(--color-brand, #5b5bf0) !important;
	color: #ffffff !important;
	border: 1px solid rgba(255, 255, 255, 0.12) !important;
	box-shadow:
		0 1px 2px 0 rgba(0, 0, 0, 0.2),
		inset 0 1px 0 0 rgba(255, 255, 255, 0.18);
	position: relative;
}

.button-frame--brand:not(:disabled):not([aria-disabled='true']):hover,
.button-frame--brand:not(:disabled):not([aria-disabled='true']):focus-visible {
	filter: brightness(1.06);
	box-shadow:
		0 3px 8px -1px rgba(0, 0, 0, 0.25),
		inset 0 1px 0 0 rgba(255, 255, 255, 0.22);
	transform: translateY(-1px);
}

.button-frame--brand:not(:disabled):not([aria-disabled='true']):active {
	filter: brightness(0.96);
	box-shadow:
		0 1px 2px 0 rgba(0, 0, 0, 0.2),
		inset 0 1px 2px 0 rgba(0, 0, 0, 0.2);
	transform: translateY(0.5px) scale(0.98);
}

.button-frame--danger {
	background: var(--color-red) !important;
	color: #ffffff !important;
	border: none !important;
	box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.35);
}

.button-frame--danger:not(:disabled):not([aria-disabled='true']):hover,
.button-frame--danger:not(:disabled):not([aria-disabled='true']):focus-visible {
	filter: brightness(1.08);
	box-shadow:
		0 4px 12px 0 rgba(239, 68, 68, 0.35),
		0 1px 3px 0 rgba(0, 0, 0, 0.35);
}

.button-frame--danger:not(:disabled):not([aria-disabled='true']):active {
	box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.25);
}

.button-frame--colored::before {
	position: absolute;
	inset: 0;
	padding: 1px;
	pointer-events: none;
	content: '';
	border-radius: inherit;
	background: linear-gradient(180deg, rgba(255, 255, 255, 0.25), rgba(255, 255, 255, 0));
	-webkit-mask:
		linear-gradient(#000 0 0) content-box,
		linear-gradient(#000 0 0);
	-webkit-mask-composite: xor;
	mask-composite: exclude;
}

.button-frame--outlined {
	box-shadow: 0 0 0 1px var(--button-color, var(--surface-5));
}

.button-frame--outlined:not(:disabled):not([aria-disabled='true']):hover,
.button-frame--outlined:not(:disabled):not([aria-disabled='true']):focus-visible {
	box-shadow:
		0 0 0 1px var(--button-color, var(--surface-5)),
		0 3px 8px -2px rgba(0, 0, 0, 0.15);
}

.button-frame--outlined:not(:disabled):not([aria-disabled='true']):active {
	box-shadow: 0 0 0 1px var(--button-color, var(--surface-5));
}

.button-frame--quiet {
	color: var(--button-color, var(--color-base));
}

.button-frame--quiet:not(:disabled):not([aria-disabled='true']):hover,
.button-frame--quiet:not(:disabled):not([aria-disabled='true']):focus-visible {
	box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.08);
}

.button-frame--quiet:not(:disabled):not([aria-disabled='true']):active {
	box-shadow: none;
}

@media (prefers-reduced-motion: reduce) {
	[data-button] {
		transition: none !important;
		transform: none !important;
	}
	[data-button]:not(:disabled):not([aria-disabled='true']):active {
		transform: none !important;
	}
	.button-frame--brand::after {
		display: none !important;
	}
}
</style>
