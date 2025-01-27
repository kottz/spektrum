<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import type { Media } from '$lib/types';
	import { cn } from '$lib/utils';
	import TableContainer from './table/table-container.svelte';
	import SearchInput from './table/search-input.svelte';
	import Pagination from './table/pagination.svelte';
	import EditableInput from './table/editable-input.svelte';

	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: '',
		isAddingMedia: false,
		editingValues: new Map<string, string | number>(),
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
		return filteredData().slice(
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

<TableContainer>
	<svelte:fragment slot="header-left">
		<SearchInput
			value={state.searchTerm}
			placeholder="Search media..."
			onInput={(value) => {
				state.searchTerm = value;
				state.currentPage = 0;
			}}
		/>
	</svelte:fragment>

	<svelte:fragment slot="header-right">
		<Button on:click={handleAddMedia}>Add Media</Button>
	</svelte:fragment>

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
						<EditableInput
							value={state.newMediaData.title}
							placeholder="Title"
							onChange={(value) => (state.newMediaData.title = value)}
							onCommit={(value) => (state.newMediaData.title = value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.artist}
							placeholder="Artist"
							onChange={(value) => (state.newMediaData.artist = value)}
							onCommit={(value) => (state.newMediaData.artist = value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							type="number"
							value={state.newMediaData.release_year.toString()}
							onChange={(value) => (state.newMediaData.release_year = parseInt(value))}
							onCommit={(value) => (state.newMediaData.release_year = parseInt(value))}
						/>
					</Table.Cell>
					<Table.Cell>0</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.spotify_uri || ''}
							placeholder="Spotify URI"
							onChange={(value) => (state.newMediaData.spotify_uri = value)}
							onCommit={(value) => (state.newMediaData.spotify_uri = value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.youtube_id || ''}
							placeholder="YouTube ID"
							onChange={(value) => (state.newMediaData.youtube_id = value)}
							onCommit={(value) => (state.newMediaData.youtube_id = value)}
						/>
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
						<EditableInput
							value={getEditingValue(media.id, 'title', media.title)}
							onChange={(value) => handleMediaFieldChange(media.id, 'title', value)}
							onCommit={(value) => commitChanges(media.id, 'title')}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={getEditingValue(media.id, 'artist', media.artist)}
							onChange={(value) => handleMediaFieldChange(media.id, 'artist', value)}
							onCommit={(value) => commitChanges(media.id, 'artist')}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							type="number"
							value={getEditingValue(media.id, 'release_year', media.release_year || '').toString()}
							onChange={(value) =>
								handleMediaFieldChange(media.id, 'release_year', parseInt(value))}
							onCommit={(value) => commitChanges(media.id, 'release_year')}
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
						<EditableInput
							value={getEditingValue(media.id, 'spotify_uri', media.spotify_uri || '')}
							placeholder="Spotify URI"
							onChange={(value) => handleMediaFieldChange(media.id, 'spotify_uri', value)}
							onCommit={(value) => commitChanges(media.id, 'spotify_uri')}
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
								<EditableInput
									value={getEditingValue(media.id, 'youtube_id', media.youtube_id || '')}
									placeholder="YouTube ID"
									onChange={(value) => handleMediaFieldChange(media.id, 'youtube_id', value)}
									onCommit={(value) => commitChanges(media.id, 'youtube_id')}
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

	<Pagination
		currentPage={state.currentPage}
		{totalPages}
		totalItems={filteredData().length}
		itemsPerPage={state.itemsPerPage}
		onPageChange={(page) => (state.currentPage = page)}
	/>
</TableContainer>
