<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import { QuestionType } from '$lib/types';

	let { show = $bindable() } = $props();

	const state = $state({
		searchTerm: '',
		show: true // Changed from prop to state since it's managed internally
	});

	const store = $derived(adminStore.getState());

	// Get unique characters from question options, but only from character-type questions
	// Memoized lookups
	const characterQuestionMap = $derived(() => {
		const map = new Set<number>();
		for (const q of store.questions) {
			if (q.question_type === QuestionType.Character) map.add(q.id);
		}
		return map;
	});

	const distinctCharacters = $derived(() => {
		const chars = new Set<string>();
		for (const opt of store.options) {
			if (characterQuestionMap().has(opt.question_id) && opt.option_text) {
				chars.add(opt.option_text);
			}
		}
		return Array.from(chars);
	});

	// Filter characters based on search term
	const filteredCharacters = $derived(() => {
		return distinctCharacters().filter((char) =>
			char.toLowerCase().includes(state.searchTerm.toLowerCase())
		);
	});

	function handleDragStart(e: DragEvent, char: string) {
		if (e.dataTransfer) {
			e.dataTransfer.setData('text/plain', char);
			const target = e.currentTarget as HTMLElement;
			target.classList.add('dragging');
		}
	}

	function handleDragEnd(e: DragEvent) {
		const target = e.currentTarget as HTMLElement;
		target.classList.remove('dragging');
	}
</script>

{#if show}
	<div class="fixed right-0 top-16 h-[calc(100vh-4rem)] w-96 border-l bg-white shadow-lg">
		<!-- Fixed header -->
		<div class="sticky top-0 space-y-4 border-b bg-white p-4">
			<div class="flex items-center justify-between">
				<h2 class="text-lg font-semibold">Character Bank</h2>
				<Button variant="outline" on:click={() => (show = false)}>Close</Button>
			</div>
			<Input type="text" placeholder="Search characters..." bind:value={state.searchTerm} />
		</div>
		<!-- Scrollable content -->
		<div class="h-[calc(100%-7rem)] overflow-y-auto p-4">
			<div class="grid grid-cols-4 gap-4">
				{#each filteredCharacters() as char}
					<div
						class="cursor-grab transition-transform hover:scale-105"
						draggable="true"
						on:dragstart={(e) => handleDragStart(e, char)}
						on:dragend={handleDragEnd}
					>
						<img src={`/img/${char}.avif`} alt={char} class="w-full rounded-lg" />
						<div class="mt-1 truncate text-center text-sm" title={char}>
							{char}
						</div>
					</div>
				{/each}
			</div>
			{#if filteredCharacters().length === 0}
				<div class="mt-4 text-center text-gray-500">No characters found</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.dragging {
		opacity: 0.5;
	}
</style>
