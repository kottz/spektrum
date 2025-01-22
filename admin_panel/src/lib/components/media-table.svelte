<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import type { Media } from '$lib/types';
	import { cn } from '$lib/utils';

	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: '',
		isAddingMedia: false,
		editingValues: new Map<string, string | number>(), // Store temporary edits
		newMediaData: {
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		} as Partial<Media>
	});

	const filteredData = $derived(() => {
		const data = adminStore.getState().media;
		return data.filter((media) => {
			const searchLower = state.searchTerm.toLowerCase();
			return (
				media.title.toLowerCase().includes(searchLower) ||
				media.artist.toLowerCase().includes(searchLower) ||
				media.id.toString().includes(state.searchTerm)
			);
		});
	});

	const totalPages = $derived(Math.ceil(filteredData().length / state.itemsPerPage));

	const paginatedData = $derived(() => {
		const filtered = filteredData();
		return filtered.slice(
			state.currentPage * state.itemsPerPage,
			(state.currentPage + 1) * state.itemsPerPage
		);
	});

	function isMediaUsed(mediaId: number): boolean {
		return adminStore.getState().questions.some((q) => q.media_id === mediaId);
	}

	function getQuestionCount(mediaId: number): number {
		return adminStore.getState().questions.filter((q) => q.media_id === mediaId).length;
	}

	function formatSpotifyUri(uri: string | null): string {
		if (!uri) return '';
		return uri.includes('spotify:') ? uri.split(':').pop() || '' : uri;
	}

	function getEditKey(id: number, field: string): string {
		return `${id}-${field}`;
	}

	function getEditingValue(
		id: number,
		field: keyof Media,
		currentValue: string | number
	): string | number {
		const editKey = getEditKey(id, field);
		return state.editingValues.has(editKey) ? state.editingValues.get(editKey)! : currentValue;
	}

	function handleMediaFieldChange(id: number, field: keyof Media, value: string | number) {
		const editKey = getEditKey(id, field);
		state.editingValues.set(editKey, value);
	}

	function commitChanges(id: number, field: keyof Media) {
		const editKey = getEditKey(id, field);
		const newValue = state.editingValues.get(editKey);

		if (newValue !== undefined) {
			adminStore.modifyEntity('media', id, { [field]: newValue });
			state.editingValues.delete(editKey);
		}
	}

	function handleKeyDown(
		event: KeyboardEvent & { currentTarget: HTMLInputElement },
		id: number,
		field: keyof Media
	) {
		if (event.key === 'Enter') {
			commitChanges(id, field);
			event.currentTarget.blur();
		}
	}

	function handleAddMedia() {
		const maxId = Math.max(0, ...adminStore.getState().media.map((m) => m.id));
		state.newMediaData = {
			id: maxId + 1,
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		};
		state.isAddingMedia = true;
	}

	function handleSaveMedia() {
		if (state.newMediaData.title && state.newMediaData.artist) {
			adminStore.addEntity('media', state.newMediaData as Media);
			state.isAddingMedia = false;
			state.newMediaData = {
				title: '',
				artist: '',
				release_year: new Date().getFullYear(),
				spotify_uri: '',
				youtube_id: ''
			};
		}
	}

	function handleCancelAdd() {
		state.isAddingMedia = false;
		state.newMediaData = {
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		};
	}

	function handleDeleteMedia(mediaId: number) {
		if (isMediaUsed(mediaId)) return;
		adminStore.deleteEntity('media', mediaId);
	}

	function nextPage() {
		if (state.currentPage < totalPages - 1) {
			state.currentPage++;
		}
	}

	function previousPage() {
		if (state.currentPage > 0) {
			state.currentPage--;
		}
	}
</script>

<div class="w-full">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-4">
			<Input
				type="text"
				placeholder="Search media..."
				bind:value={state.searchTerm}
				oninput={() => (state.currentPage = 0)}
				class="max-w-sm"
			/>
		</div>
		<Button on:click={handleAddMedia}>Add Media</Button>
	</div>

	<div class="rounded-md border">
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>ID</Table.Head>
					<Table.Head>Title</Table.Head>
					<Table.Head>Artist</Table.Head>
					<Table.Head>Release Year</Table.Head>
					<Table.Head>Questions</Table.Head>
					<Table.Head>Spotify</Table.Head>
					<Table.Head>YouTube</Table.Head>
					<Table.Head class="text-right">Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#if state.isAddingMedia}
					<Table.Row class="bg-blue-50">
						<Table.Cell>{state.newMediaData.id}</Table.Cell>
						<Table.Cell>
							<Input bind:value={state.newMediaData.title} placeholder="Title" />
						</Table.Cell>
						<Table.Cell>
							<Input bind:value={state.newMediaData.artist} placeholder="Artist" />
						</Table.Cell>
						<Table.Cell>
							<Input type="number" bind:value={state.newMediaData.release_year} />
						</Table.Cell>
						<Table.Cell>0</Table.Cell>
						<Table.Cell>
							<Input bind:value={state.newMediaData.spotify_uri} placeholder="Spotify URI" />
						</Table.Cell>
						<Table.Cell>
							<Input bind:value={state.newMediaData.youtube_id} placeholder="YouTube ID" />
						</Table.Cell>
						<Table.Cell class="text-right">
							<div class="flex justify-end gap-2">
								<Button variant="outline" size="sm" on:click={handleSaveMedia}>Save</Button>
								<Button
									variant="outline"
									size="sm"
									class="text-red-600 hover:bg-red-50"
									on:click={handleCancelAdd}
								>
									Cancel
								</Button>
							</div>
						</Table.Cell>
					</Table.Row>
				{/if}

				{#each paginatedData() as media (media.id)}
					<Table.Row
						class={cn(
							'transition-colors',
							isMediaUsed(media.id) ? 'bg-gray-50 hover:bg-gray-50' : 'hover:bg-gray-50'
						)}
					>
						<Table.Cell>{media.id}</Table.Cell>
						<Table.Cell>
							<Input
								value={getEditingValue(media.id, 'title', media.title)}
								on:input={(e) => handleMediaFieldChange(media.id, 'title', e.currentTarget.value)}
								on:blur={() => commitChanges(media.id, 'title')}
								on:keydown={(e) => handleKeyDown(e, media.id, 'title')}
							/>
						</Table.Cell>
						<Table.Cell>
							<Input
								value={getEditingValue(media.id, 'artist', media.artist)}
								on:input={(e) => handleMediaFieldChange(media.id, 'artist', e.currentTarget.value)}
								on:blur={() => commitChanges(media.id, 'artist')}
								on:keydown={(e) => handleKeyDown(e, media.id, 'artist')}
							/>
						</Table.Cell>
						<Table.Cell>
							<Input
								type="number"
								value={getEditingValue(media.id, 'release_year', media.release_year || '')}
								on:input={(e) =>
									handleMediaFieldChange(media.id, 'release_year', parseInt(e.currentTarget.value))}
								on:blur={() => commitChanges(media.id, 'release_year')}
								on:keydown={(e) => handleKeyDown(e, media.id, 'release_year')}
							/>
						</Table.Cell>
						<Table.Cell>
							<span
								class="inline-flex rounded-full bg-blue-100 px-2 py-1 text-xs font-semibold text-blue-800"
							>
								{getQuestionCount(media.id)}
							</span>
						</Table.Cell>
						<Table.Cell>
							<Input
								value={getEditingValue(media.id, 'spotify_uri', media.spotify_uri || '')}
								placeholder="Spotify URI"
								on:input={(e) =>
									handleMediaFieldChange(media.id, 'spotify_uri', e.currentTarget.value)}
								on:blur={() => commitChanges(media.id, 'spotify_uri')}
								on:keydown={(e) => handleKeyDown(e, media.id, 'spotify_uri')}
							/>
							{#if media.spotify_uri}
								<a
									href={`https://open.spotify.com/track/${formatSpotifyUri(media.spotify_uri)}`}
									target="_blank"
									class="mt-1 block text-xs text-blue-600 hover:underline"
								>
									Open in Spotify
								</a>
							{/if}
						</Table.Cell>
						<Table.Cell>
							<div class="flex items-center gap-2">
								<div>
									<Input
										value={getEditingValue(media.id, 'youtube_id', media.youtube_id || '')}
										placeholder="YouTube ID"
										on:input={(e) =>
											handleMediaFieldChange(media.id, 'youtube_id', e.currentTarget.value)}
										on:blur={() => commitChanges(media.id, 'youtube_id')}
										on:keydown={(e) => handleKeyDown(e, media.id, 'youtube_id')}
									/>
									{#if media.youtube_id}
										<a
											href={`https://youtube.com/watch?v=${media.youtube_id}`}
											target="_blank"
											class="mt-1 block text-xs text-blue-600 hover:underline"
										>
											Open in YouTube
										</a>
									{/if}
								</div>
								{#if media.youtube_id}
									<img
										class="mb-5 h-9 w-16"
										src={`https://i.ytimg.com/vi_webp/${media.youtube_id}/default.webp`}
										alt={`YouTube thumbnail for ${media.title}`}
									/>
								{/if}
							</div>
						</Table.Cell>
						<Table.Cell class="text-right">
							<div class="flex justify-end gap-2">
								<Button
									variant="outline"
									size="sm"
									class="text-red-600 hover:bg-red-50"
									on:click={() => handleDeleteMedia(media.id)}
									disabled={isMediaUsed(media.id)}
									title={isMediaUsed(media.id) ? 'Cannot delete media used in questions' : ''}
								>
									Delete
								</Button>
							</div>
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	</div>

	<div class="mt-4 flex items-center justify-between">
		<div class="text-sm text-muted-foreground">
			Showing {state.currentPage * state.itemsPerPage + 1} to {Math.min(
				(state.currentPage + 1) * state.itemsPerPage,
				filteredData().length
			)} of {filteredData().length} media entries
		</div>
		<div class="flex gap-2">
			<Button
				variant="outline"
				size="sm"
				on:click={previousPage}
				disabled={state.currentPage === 0}
			>
				Previous
			</Button>
			<Button
				variant="outline"
				size="sm"
				on:click={nextPage}
				disabled={state.currentPage >= totalPages - 1}
			>
				Next
			</Button>
		</div>
	</div>
</div>
