<script setup lang="ts">
import { SpinnerIcon } from '@modrinth/assets'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, nextTick, useTemplateRef } from 'vue'

import { GradleNewsError } from '@/helpers/gradle/news'
import { injectGradleNews } from '@/providers/gradle-news'

import GradleNewsCard from './gradle-news-card.vue'

const props = defineProps<{
	page: number
	pageSize: number
}>()

const { formatMessage } = useVIntl()
const { articles, error, isPending, isError } = injectGradleNews()
const content = useTemplateRef<HTMLElement>('content')
const pageArticles = computed(() =>
	articles.value?.slice((props.page - 1) * props.pageSize, props.page * props.pageSize),
)

async function scrollToTop() {
	await nextTick()
	content.value?.closest('[data-modal-content]')?.scrollTo({ top: 0 })
}

defineExpose({ scrollToTop })

const isRateLimited = computed(
	() => error.value instanceof GradleNewsError && error.value.httpStatus === 429,
)

const messages = defineMessages({
	loading: {
		id: 'gradle.app.news.loading',
		defaultMessage: 'Loading Gradle news…',
	},
	empty: {
		id: 'gradle.app.news.empty',
		defaultMessage: 'No Gradle news yet.',
	},
	error: {
		id: 'gradle.app.news.error',
		defaultMessage: 'Could not load Gradle news.',
	},
	rateLimited: {
		id: 'gradle.app.news.rate-limited',
		defaultMessage: 'Too many requests.',
	},
})
</script>

<template>
	<div ref="content" class="flex min-w-0 flex-col gap-6 pb-5">
		<div
			v-if="isPending"
			role="status"
			class="flex items-center justify-center gap-2 py-8 text-secondary"
		>
			<SpinnerIcon aria-hidden="true" class="h-5 w-5 shrink-0 animate-spin" />
			{{ formatMessage(messages.loading) }}
		</div>
		<template v-else>
			<p v-if="isError" role="alert" class="m-0 text-secondary">
				{{ formatMessage(isRateLimited ? messages.rateLimited : messages.error) }}
			</p>

			<template v-if="articles?.length">
				<GradleNewsCard
					v-for="(article, index) in pageArticles"
					:key="`${article.id}-${(page - 1) * pageSize + index}`"
					:article="article"
				/>
			</template>
			<p v-else-if="!isError" role="status" class="m-0 py-8 text-center text-secondary">
				{{ formatMessage(messages.empty) }}
			</p>
		</template>
	</div>
</template>
