<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import UploadDialog from '$lib/components/character-upload-dialog.svelte';

	let { className = '' } = $props();

	const state = $state({
		searchTerm: ''
	});

	const store = $derived(adminStore.getState());

	const filteredCharacters = $derived(() => {
		const searchLower = state.searchTerm.toLowerCase();

		return store.characters
			.filter((c) => c.name.toLowerCase().includes(searchLower))
			.map((c) => ({
				...c,
				// Use spread operator to maintain object reference when no changes
				...(c._pendingImage
					? {
							image_url: c._pendingImage.dataUrl,
							isPending: true
						}
					: {
							image_url: c.image_url,
							isPending: false
						})
			}));
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

<div class={`rounded-md border bg-white ${className} flex h-full flex-col`}>
	<!-- Header -->
	<div class="flex-none border-b p-4">
		<div class="mb-4 flex items-center justify-between">
			<h2 class="w-48 text-lg font-semibold">Character Bank</h2>
			<Input
				class="mr-4"
				type="text"
				placeholder="Search characters..."
				bind:value={state.searchTerm}
			/>
			<UploadDialog />
		</div>
	</div>

	<!-- Content -->
	<ScrollArea class="flex-1">
		<div class="grid grid-cols-12 gap-4 p-4" role="listbox" aria-label="Available characters">
			{#each filteredCharacters() as char (char.id)}
				<div
					class={`cursor-grab transition-transform hover:scale-105 ${char.isPending ? 'ring-2 ring-green-500' : ''}`}
					draggable="true"
					role="option"
					tabindex="0"
					aria-selected="false"
					aria-label={`Drag ${char.name} character`}
					ondragstart={(e) => handleDragStart(e, char.name)}
					ondragend={handleDragEnd}
				>
					{#if char.image_url?.endsWith('.webm')}
						<video src={char.image_url} class="w-full rounded-lg" autoplay loop muted />
					{:else}
						<img src={char.image_url} alt={char.name} class="w-full rounded-lg" loading="lazy" />
					{/if}
					<div class="mt-1 truncate text-center text-sm" title={char.name}>
						{char.name}
						{#if char.isPending}
							<span class="ml-1 text-xs text-gray-500">(pending)</span>
						{/if}
					</div>
				</div>
			{/each}
			{#if filteredCharacters().length === 0}
				<div class="col-span-12 mt-4 text-center text-gray-500">No characters found</div>
			{/if}
		</div>
	</ScrollArea>
</div>
