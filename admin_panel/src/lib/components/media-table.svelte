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
		} as Partial<Media>,
		sortKey: 'id' as 'id' | 'title' | 'artist' | 'release_year',
		sortDirection: 'asc' as 'asc' | 'desc'
	});

	const filteredData = $derived(() => {
		const data = adminStore.getState().media;
		const filtered = data.filter((media) => {
			const searchLower = state.searchTerm.toLowerCase();
			return (
				media.title.toLowerCase().includes(searchLower) ||
				media.artist.toLowerCase().includes(searchLower) ||
				media.id.toString().includes(state.searchTerm)
			);
		});

		return filtered.slice().sort((a, b) => {
			if (state.sortKey === 'id') {
				return state.sortDirection === 'asc' ? a.id - b.id : b.id - a.id;
			} else if (state.sortKey === 'title') {
				const titleA = a.title?.toLowerCase() || 'zzz_unknown';
				const titleB = b.title?.toLowerCase() || 'zzz_unknown';
				const comparison = titleA.localeCompare(titleB);
				return state.sortDirection === 'asc' ? comparison : -comparison;
			} else if (state.sortKey === 'artist') {
				const artistA = a.artist?.toLowerCase() || 'zzz_unknown';
				const artistB = b.artist?.toLowerCase() || 'zzz_unknown';
				const comparison = artistA.localeCompare(artistB);
				return state.sortDirection === 'asc' ? comparison : -comparison;
			} else if (state.sortKey === 'release_year') {
				const yearA = a.release_year || 0;
				const yearB = b.release_year || 0;
				return state.sortDirection === 'asc' ? yearA - yearB : yearB - yearA;
			}
			return 0;
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

	function parseSpotifyLink(link: string): string {
		// Check for full URL format (e.g., https://open.spotify.com/track/xyz)
		const spotifyUrlRegex = /(?:https?:\/\/)?(?:open\.)?spotify\.com\/track\/([a-zA-Z0-9]+)/;
		const urlMatch = link.match(spotifyUrlRegex);
		if (urlMatch && urlMatch[1]) {
			return urlMatch[1];
		}
		// Check for spotify URI format (e.g., spotify:track:xyz)
		const spotifyUriRegex = /spotify:track:([a-zA-Z0-9]+)/;
		const uriMatch = link.match(spotifyUriRegex);
		if (uriMatch && uriMatch[1]) {
			return uriMatch[1];
		}
		// Otherwise, if not a recognizable full link, return original
		return link;
	}

	function parseYoutubeLink(link: string): string {
		// Format: Share link or mobile share link, e.g.,
		//   https://youtu.be/VIDEOID?si=...
		//   https://m.youtu.be/VIDEOID?si=...
		let match = link.match(/(?:https?:\/\/)?(?:www\.)?(?:m\.)?youtu\.be\/([^?&]+)/);
		if (match && match[1]) {
			return match[1];
		}
		// Format: Standard URL, e.g., https://www.youtube.com/watch?v=VIDEOID
		match = link.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/watch\?v=([^&]+)/);
		if (match && match[1]) {
			return match[1];
		}
		// Format: Embed URL, e.g., https://www.youtube.com/embed/VIDEOID
		match = link.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/embed\/([^?&]+)/);
		if (match && match[1]) {
			return match[1];
		}
		// Otherwise, return the original input.
		return link;
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
		let newValue = state.editingValues.get(editKey);

		if (newValue !== undefined) {
			if (field === 'spotify_uri' && typeof newValue === 'string') {
				// If a full link is detected, extract the track ID.
				newValue = parseSpotifyLink(newValue);
			}
			if (field === 'youtube_id' && typeof newValue === 'string') {
				// If a full link is detected, extract the video ID.
				newValue = parseYoutubeLink(newValue);
			}
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

	function handleSort(key: 'id' | 'title' | 'artist' | 'release_year') {
		if (state.sortKey === key) {
			state.sortDirection = state.sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			state.sortKey = key;
			state.sortDirection = 'asc';
		}
		state.currentPage = 0; // Reset to first page when changing sort
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
				<Table.Head>
					<div class="flex items-center gap-1">
						ID
						<Button variant="ghost" size="sm" on:click={() => handleSort('id')}>
							{#if state.sortKey === 'id'}
								{state.sortDirection === 'asc' ? '↑' : '↓'}
							{:else}
								↕
							{/if}
						</Button>
					</div>
				</Table.Head>
				<Table.Head>
					<div class="flex items-center gap-1">
						Title
						<Button variant="ghost" size="sm" on:click={() => handleSort('title')}>
							{#if state.sortKey === 'title'}
								{state.sortDirection === 'asc' ? '↑' : '↓'}
							{:else}
								↕
							{/if}
						</Button>
					</div>
				</Table.Head>
				<Table.Head>
					<div class="flex items-center gap-1">
						Artist
						<Button variant="ghost" size="sm" on:click={() => handleSort('artist')}>
							{#if state.sortKey === 'artist'}
								{state.sortDirection === 'asc' ? '↑' : '↓'}
							{:else}
								↕
							{/if}
						</Button>
					</div>
				</Table.Head>
				<Table.Head>
					<div class="flex items-center gap-1">
						Release Year
						<Button variant="ghost" size="sm" on:click={() => handleSort('release_year')}>
							{#if state.sortKey === 'release_year'}
								{state.sortDirection === 'asc' ? '↑' : '↓'}
							{:else}
								↕
							{/if}
						</Button>
					</div>
				</Table.Head>
				<Table.Head>Questions</Table.Head>
				<Table.Head>Spotify</Table.Head>
				<Table.Head>YouTube</Table.Head>
				<Table.Head class="text-right">Actions</Table.Head>
			</Table.Row>
		</Table.Header>
		<Table.Body>
			{#if state.isAddingMedia}
				<Table.Row class="bg-blue-50 dark:bg-gray-800">
					<Table.Cell>{state.newMediaData.id}</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.title || ''}
							placeholder="Title"
							onChange={(value) => (state.newMediaData.title = value)}
							onCommit={(value) => (state.newMediaData.title = value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.artist || ''}
							placeholder="Artist"
							onChange={(value) => (state.newMediaData.artist = value)}
							onCommit={(value) => (state.newMediaData.artist = value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							type="number"
							value={state.newMediaData.release_year?.toString() || ''}
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
							onCommit={(value) => (state.newMediaData.spotify_uri = parseSpotifyLink(value))}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.youtube_id || ''}
							placeholder="YouTube ID"
							onChange={(value) => (state.newMediaData.youtube_id = value)}
							onCommit={(value) => (state.newMediaData.youtube_id = parseYoutubeLink(value))}
						/>
					</Table.Cell>
					<Table.Cell class="text-right">
						<div class="flex justify-end gap-2">
							<Button variant="outline" size="sm" on:click={handleSaveMedia}>Save</Button>
							<Button
								variant="outline"
								size="sm"
								class="text-red-600 hover:bg-red-50 hover:dark:bg-red-800"
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
						isMediaUsed(media.id)
							? 'hover:bg-gray-50 dark:hover:bg-gray-800'
							: 'hover:bg-gray-50 dark:hover:bg-gray-800'
					)}
				>
					<Table.Cell>{media.id}</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={getEditingValue(media.id, 'title', media.title).toString()}
							onChange={(value) => handleMediaFieldChange(media.id, 'title', value)}
							onCommit={(value) => commitChanges(media.id, 'title')}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={getEditingValue(media.id, 'artist', media.artist).toString()}
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
							value={getEditingValue(media.id, 'spotify_uri', media.spotify_uri || '').toString()}
							placeholder="Spotify URI"
							onChange={(value) => handleMediaFieldChange(media.id, 'spotify_uri', value)}
							onCommit={(value) => commitChanges(media.id, 'spotify_uri')}
						/>
						{#if media.spotify_uri}
							<a
								href={`https://open.spotify.com/track/${parseSpotifyLink(media.spotify_uri)}`}
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
									value={getEditingValue(media.id, 'youtube_id', media.youtube_id || '').toString()}
									placeholder="YouTube ID"
									onChange={(value) => handleMediaFieldChange(media.id, 'youtube_id', value)}
									onCommit={(value) => commitChanges(media.id, 'youtube_id')}
								/>
								{#if media.youtube_id}
									<a
										href={`https://youtube.com/watch?v=${parseYoutubeLink(media.youtube_id)}`}
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
									src={`https://i.ytimg.com/vi_webp/${parseYoutubeLink(media.youtube_id)}/default.webp`}
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
