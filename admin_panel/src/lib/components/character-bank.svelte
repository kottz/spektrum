<script lang="ts">
	import type { Character } from '$lib/types';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/data-manager.svelte';

	let { show = $bindable() } = $props();

	const state = $state({
		searchTerm: '',
		show: true
	});

	const store = $derived(adminStore.getState());

	const filteredCharacters = $derived(() => {
		return store.characters.filter((char: Character) =>
			char.name.toLowerCase().includes(state.searchTerm.toLowerCase())
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
			<div class="grid grid-cols-4 gap-4" role="listbox" aria-label="Available characters">
				{#each filteredCharacters() as char}
					<div
						class="cursor-grab transition-transform hover:scale-105"
						draggable="true"
						role="option"
						tabindex="0"
						aria-selected="false"
						aria-label={`Drag ${char.name} character`}
						ondragstart={(e) => handleDragStart(e, char.name)}
						ondragend={handleDragEnd}
					>
						<img src={char.image_url} alt={char.name} class="w-full rounded-lg" />
						<div class="mt-1 truncate text-center text-sm" title={char.name}>
							{char.name}
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
