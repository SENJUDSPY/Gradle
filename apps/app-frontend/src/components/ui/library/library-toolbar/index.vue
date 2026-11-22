<script setup lang="ts">
import { PlusIcon, SearchIcon, SquarePlusIcon } from '@modrinth/assets'
import { Button, defineMessages, Input, useVIntl } from '@modrinth/ui'
import { computed, inject, onMounted, onUnmounted, ref } from 'vue'

import FilterMenu from '@/components/ui/library/library-toolbar/filter-menu.vue'
import NewGroupModal from '@/components/ui/library/library-toolbar/new-group-modal.vue'
import SortMenu from '@/components/ui/library/library-toolbar/sort-menu.vue'
import { useLibrary } from '@/components/ui/library/use-library'

const { search, selectedLibraryInstances, openNewGroupModal } = useLibrary()
const showCreationModal = inject<() => void>('showCreationModal')
const { formatMessage } = useVIntl()
const messages = defineMessages({
	search: { id: 'app.library.search.placeholder', defaultMessage: 'Search' },
	newGroup: { id: 'app.library.group.new', defaultMessage: 'New group' },
	newInstance: { id: 'app.library.instance.new', defaultMessage: 'New instance' },
})
const selectedInstanceIds = computed(
	() =>
		new Set([...selectedLibraryInstances.value.values()].map((selection) => selection.instanceId)),
)

const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.platform)
const searchInputRef = ref()

function handleKeyDown(e: KeyboardEvent) {
	if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
		e.preventDefault()
		const inputEl =
			searchInputRef.value?.element?.querySelector?.('input') ??
			(document.querySelector('input[placeholder*="Search"]') as HTMLInputElement | null)
		inputEl?.focus()
		inputEl?.select()
	}
}

onMounted(() => {
	window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
	window.removeEventListener('keydown', handleKeyDown)
})

function openNewGroup() {
	openNewGroupModal(selectedInstanceIds.value)
}
</script>

<template>
	<div class="flex flex-col gap-2">
		<div class="flex flex-wrap gap-2">
			<div class="relative min-w-[16rem] flex-1">
				<Input
					ref="searchInputRef"
					v-model="search"
					:icon="SearchIcon"
					type="text"
					:placeholder="formatMessage(messages.search)"
					clearable
					wrapper-class="w-full"
				/>
				<div
					v-if="!search"
					class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 flex items-center"
				>
					<kbd
						class="hidden sm:inline-flex items-center justify-center h-5 px-1.5 text-[10px] font-semibold text-secondary bg-surface-2 rounded border border-surface-5 shadow-xs"
					>
						{{ isMac ? '⌘K' : 'Ctrl+K' }}
					</kbd>
				</div>
			</div>
			<Button @click="openNewGroup">
				<SquarePlusIcon />
				{{ formatMessage(messages.newGroup) }}
			</Button>
			<Button type="colored" color="brand" @click="showCreationModal?.()">
				<PlusIcon />
				{{ formatMessage(messages.newInstance) }}
			</Button>
		</div>
		<div class="flex flex-wrap items-center gap-2">
			<SortMenu />
			<div class="mx-2 h-6 w-px bg-surface-5" />
			<FilterMenu />
		</div>
	</div>
	<NewGroupModal />
</template>
