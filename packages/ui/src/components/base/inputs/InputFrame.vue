<template>
	<div
		v-bind="$attrs"
		class="group/input min-w-0 touch-manipulation border border-solid font-medium text-primary shadow-sm transition-[background-color,border-color,box-shadow,color] duration-150 focus-within:text-contrast focus-within:border-brand-accent focus-within:ring-2 focus-within:ring-[--color-focus-ring]/40"
		:class="[
			multiline ? 'flex w-full items-start gap-2 rounded-lg px-3 py-2' : sizeClass,
			appearanceClass,
			disabled ? 'cursor-not-allowed opacity-50' : '',
		]"
		:data-disabled="disabled || undefined"
		:data-invalid="invalid || undefined"
	>
		<slot name="leading" />
		<slot />
		<slot name="trailing" />
	</div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { InputAppearance, InputSize } from './types'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
	defineProps<{
		appearance?: InputAppearance
		disabled?: boolean
		invalid?: boolean
		multiline?: boolean
		size?: InputSize
	}>(),
	{
		appearance: 'surface',
		disabled: false,
		invalid: false,
		multiline: false,
		size: 'standard',
	},
)

const sizeClass = computed(
	() =>
		({
			small: 'inline-flex h-9 min-h-[36px] items-center gap-1.5 rounded-lg px-3',
			standard: 'inline-flex h-10 min-h-[40px] items-center gap-2 rounded-lg px-3.5',
			medium: 'inline-flex h-11 min-h-[44px] items-center gap-2 rounded-lg px-4',
			large: 'inline-flex h-12 min-h-[48px] items-center gap-2 rounded-lg px-4',
		})[props.size],
)

const appearanceClass = computed(() => {
	if (props.invalid) return 'border-red bg-red/10 text-red focus-within:border-red focus-within:ring-red/30'
	if (props.appearance === 'transparent') return 'border-transparent bg-transparent'
	if (props.appearance === 'button') return 'border-surface-5 bg-surface-3'
	return 'border-surface-5 bg-surface-2 text-contrast hover:border-surface-5/80'
})
</script>
<style scoped>
:deep(input) {
	border-radius: 0;
}
</style>
