<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import UploadDialog from '$lib/components/character-upload-dialog.svelte';

	let { className = '' } = $props();

	const state = $state({
		searchTerm: '',
		hoveredCharacter: null as number | null,
		cachedUsage: new Set<number>() // Set of character IDs that are in use
	});

	const store = $derived(adminStore.getState());

	function isCharacterUsed(characterId: number): boolean {
		if (state.cachedUsage.has(characterId)) {
			return true;
		}

		const character = store.characters.find((c) => c.id === characterId);
		if (!character || state.hoveredCharacter !== characterId) return false;

		const isUsed = store.options.some((opt) => opt.option_text === character.name);

		if (isUsed) {
			state.cachedUsage.add(characterId);
		}
		return isUsed;
	}

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

	function deleteCharacter(id: number) {
		adminStore.deleteEntity('characters', id);
		state.cachedUsage.delete(id);
	}

	// Basic search without complex transformations
	const filteredCharacters = $derived(() => {
		const searchLower = state.searchTerm.toLowerCase();
		return store.characters
			.filter((c) => c.name.toLowerCase().includes(searchLower))
			.map((c) => ({
				...c,
				image_url: c._pendingImage?.dataUrl || c.image_url,
				isPending: !!c._pendingImage
			}));
	});
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
					class="group relative cursor-grab transition-transform hover:scale-105"
					draggable="true"
					role="option"
					tabindex="0"
					aria-selected="false"
					aria-label={`Drag ${char.name} character`}
					ondragstart={(e) => handleDragStart(e, char.name)}
					ondragend={handleDragEnd}
					onmouseenter={() => (state.hoveredCharacter = char.id)}
					onmouseleave={() => (state.hoveredCharacter = null)}
				>
					{#if char.image_url?.endsWith('.webm')}
						<video
							src={PUBLIC_SPEKTRUM_CDN_URL
								? `${PUBLIC_SPEKTRUM_CDN_URL}/${char.image_url}`
								: char.image_url}
							class="w-full rounded-lg"
							autoplay
							loop
							muted
						></video>
					{:else}
						<img
							src={PUBLIC_SPEKTRUM_CDN_URL
								? `${PUBLIC_SPEKTRUM_CDN_URL}/${char.image_url}`
								: char.image_url}
							alt={char.name}
							class="w-full rounded-lg"
							loading="lazy"
						/>
					{/if}

					<div class="mt-1 truncate text-center text-sm" title={char.name}>
						{char.name}
						{#if char.isPending}
							<span class="ml-1 text-xs text-gray-500">(pending)</span>
						{/if}
					</div>

					{#if state.hoveredCharacter === char.id}
						{#if !isCharacterUsed(char.id)}
							<button
								class="absolute -right-2 -top-2 flex h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white"
								onclick={(e) => {
									e.stopPropagation();
									deleteCharacter(char.id);
								}}
							>
								<span class="mb-0.5">×</span>
							</button>
						{:else}
							<div
								class="absolute -right-2 -top-2 flex h-5 w-5 items-center justify-center rounded-full bg-gray-500 text-xs text-white"
							>
								<span class="mt-px">×</span>
							</div>
						{/if}
					{/if}
				</div>
			{/each}
			{#if filteredCharacters().length === 0}
				<div class="col-span-12 mt-4 text-center text-gray-500">No characters found</div>
			{/if}
		</div>
	</ScrollArea>
</div>
