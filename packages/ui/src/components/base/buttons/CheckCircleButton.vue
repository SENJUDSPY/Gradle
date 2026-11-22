<script setup lang="ts">
import { CheckIcon } from '@modrinth/assets'
import { computed, ref } from 'vue'

import ButtonFrame from './ButtonFrame.vue'
import type { ButtonNativeType } from './types'

withDefaults(
	defineProps<{
		checked: boolean
		disabled?: boolean
		nativeType?: ButtonNativeType
	}>(),
	{
		disabled: false,
		nativeType: 'button',
	},
)

const frame = ref<InstanceType<typeof ButtonFrame> | null>(null)
const element = computed(() => frame.value?.element ?? null)

defineExpose({ element })
</script>

<template>
	<ButtonFrame
		ref="frame"
		as="button"
		type="quiet"
		size="lg"
		interaction="none"
		:disabled="disabled"
		:native-type="nativeType"
		role="radio"
		:aria-checked="checked"
		class="w-full !justify-between !gap-4 !whitespace-normal !border !border-solid !px-2 !text-left !transition-colors"
		:class="
			checked
				? '!border-brand !bg-brand-highlight !text-contrast'
				: '!border-transparent !bg-transparent !text-contrast enabled:hover:!bg-surface-3'
		"
	>
		<span class="flex min-w-0 flex-1 items-center gap-2">
			<slot />
		</span>
		<span
			v-if="checked"
			class="flex size-6 shrink-0 items-center justify-center rounded-full bg-brand text-brand-inverted check-circle-badge"
		>
			<CheckIcon class="size-4 check-circle-icon" aria-hidden="true" />
		</span>
		<span v-else class="size-6 shrink-0 rounded-full border border-solid border-surface-5" />
	</ButtonFrame>
</template>

<style scoped>
.check-circle-badge {
	animation: check-circle-pop 180ms var(--ease-bounce, cubic-bezier(0.175, 0.885, 0.32, 1.275))
		forwards;
}

@keyframes check-circle-pop {
	0% {
		transform: scale(0.5);
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
	.check-circle-badge {
		animation: none !important;
	}
}
</style>
