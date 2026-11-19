<script lang="ts">
	import * as Table from '#lib/components/ui/table/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Textarea } from '#lib/components/ui/textarea/index.ts';
	import { adminStore } from '#lib/stores/data-manager.svelte.ts';
	import type { Media } from '#lib/generated/Media.ts';
	import { cn } from '#lib/utils.ts';
	import TableContainer from './table/table-container.svelte';
	import SearchInput from './table/search-input.svelte';
	import Pagination from './table/pagination.svelte';
	import EditableInput from './table/editable-input.svelte';

	// ----- State -----
	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: '',
		isAddingMedia: false,
		isAddingBulkMedia: false,
		newMediaData: {
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		} as Partial<Media>,
		bulkMediaInput: '',
		sortKey: 'id' as 'id' | 'title' | 'artist' | 'release_year',
		sortDirection: 'asc' as 'asc' | 'desc'
	});

	// ----- Derived State -----
	const filteredData = $derived.by(() => {
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

	const totalPages = $derived(Math.ceil(filteredData.length / state.itemsPerPage));

	const paginatedData = $derived.by(() => {
		return filteredData.slice(
			state.currentPage * state.itemsPerPage,
			(state.currentPage + 1) * state.itemsPerPage
		);
	});

	// ----- Utility Functions -----
	function isMediaUsed(mediaId: number): boolean {
		return adminStore.getState().questions.some((q) => q.media_id === mediaId);
	}

	function getQuestionCount(mediaId: number): number {
		return adminStore.getState().questions.filter((q) => q.media_id === mediaId).length;
	}

	function parseSpotifyLink(link: string): string {
		if (!link) return '';
		const spotifyUrlRegex = /(?:https?:\/\/)?(?:open\.)?spotify\.com\/track\/([a-zA-Z0-9]+)/;
		const urlMatch = link.match(spotifyUrlRegex);
		if (urlMatch && urlMatch[1]) {
			return urlMatch[1];
		}
		const spotifyUriRegex = /spotify:track:([a-zA-Z0-9]+)/;
		const uriMatch = link.match(spotifyUriRegex);
		if (uriMatch && uriMatch[1]) {
			return uriMatch[1];
		}
		return link.trim();
	}

	function parseYoutubeLink(link: string): string {
		if (!link) return '';
		let match = link.match(/(?:https?:\/\/)?(?:www\.)?(?:m\.)?youtu\.be\/([^?&/\s]+)/);
		if (match && match[1]) {
			return match[1];
		}
		match = link.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/watch\?v=([^&/\s]+)/);
		if (match && match[1]) {
			return match[1];
		}
		match = link.match(/(?:https?:\/\/)?(?:www\.)?youtube\.com\/embed\/([^?&/\s]+)/);
		if (match && match[1]) {
			return match[1];
		}
		if (link.match(/^[a-zA-Z0-9_-]{11}$/)) {
			return link.trim();
		}
		return '';
	}

	// ----- Edit Handlers -----
	// The input holds the draft; a field is written to the store only on commit
	type EditableMediaField = 'title' | 'artist' | 'release_year' | 'spotify_uri' | 'youtube_id';

	function parseMediaField(field: EditableMediaField, raw: string) {
		switch (field) {
			case 'spotify_uri':
				return parseSpotifyLink(raw);
			case 'youtube_id':
				return parseYoutubeLink(raw);
			case 'release_year':
				return parseInt(raw) || new Date().getFullYear();
			default:
				return raw;
		}
	}

	function commitMediaField(id: number, field: EditableMediaField, raw: string) {
		adminStore.modifyEntity('media', id, { [field]: parseMediaField(field, raw) });
	}

	// ----- Single Add Handlers -----
	function handleAddMedia() {
		state.isAddingBulkMedia = false;
		state.bulkMediaInput = '';
		state.newMediaData = {
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		};
		state.isAddingMedia = true;
	}

	function handleSaveMedia() {
		if (state.newMediaData.title?.trim() && state.newMediaData.artist?.trim()) {
			const finalData: Omit<Media, 'id'> = {
				...state.newMediaData,
				title: state.newMediaData.title.trim(),
				artist: state.newMediaData.artist.trim(),
				release_year: state.newMediaData.release_year || new Date().getFullYear(),
				spotify_uri: parseSpotifyLink(state.newMediaData.spotify_uri || ''),
				youtube_id: parseYoutubeLink(state.newMediaData.youtube_id || '')
			} as Omit<Media, 'id'>;

			adminStore.addEntity('media', finalData);
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

	// ----- Bulk Add Handlers -----
	function handleAddBulkMedia() {
		state.isAddingMedia = false;
		state.newMediaData = {
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		};
		state.bulkMediaInput = '';
		state.isAddingBulkMedia = true;
	}

	function handleSaveBulkMedia() {
		const lines = state.bulkMediaInput.split('\n').filter((line) => line.trim() !== '');
		if (lines.length === 0) {
			return;
		}

		const newMediaItems: Omit<Media, 'id'>[] = [];

		for (const [i, line] of lines.entries()) {
			const rejectLine = (message: string) => alert(`Line ${i + 1}: ${message}`);
			const fields = line.split(',').map((f) => f.trim());

			if (fields.length !== 5) {
				rejectLine(
					`Expected 5 fields (Title, Artist, Year, Spotify, YouTube), but got ${fields.length}.`
				);
				return;
			}

			const [title, artist, yearStr, spotifyInput, youtubeInput] = fields;

			if (!title || !artist) {
				rejectLine('Title and Artist cannot be empty.');
				return;
			}

			const release_year = parseInt(yearStr);
			if (isNaN(release_year) || release_year < 1000 || release_year > 3000) {
				rejectLine(`Invalid Release Year: ${yearStr}. Must be a valid number.`);
				return;
			}

			newMediaItems.push({
				title,
				artist,
				release_year,
				spotify_uri: parseSpotifyLink(spotifyInput),
				youtube_id: parseYoutubeLink(youtubeInput)
			});
		}

		// One import is one undoable step
		adminStore.startBatch();
		newMediaItems.forEach((item) => adminStore.addEntity('media', item));
		adminStore.commitBatch();
		state.isAddingBulkMedia = false;
		state.bulkMediaInput = '';
	}

	function handleCancelBulkAdd() {
		state.isAddingBulkMedia = false;
		state.bulkMediaInput = '';
	}

	// ----- Other Actions -----
	function handleDeleteMedia(mediaId: number) {
		if (isMediaUsed(mediaId)) {
			alert('Cannot delete media that is currently used in questions.');
			return;
		}
		adminStore.deleteEntity('media', mediaId);
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
	{#snippet headerLeft()}
		<SearchInput
			value={state.searchTerm}
			placeholder="Search media by Title, Artist, or ID..."
			onInput={(value) => {
				state.searchTerm = value;
				state.currentPage = 0;
			}}
		/>
	{/snippet}

	{#snippet headerRight()}
		<div class="flex gap-2">
			<Button onclick={handleAddMedia} disabled={state.isAddingMedia || state.isAddingBulkMedia}
				>Add Media</Button
			>
			<Button
				variant="secondary"
				onclick={handleAddBulkMedia}
				disabled={state.isAddingMedia || state.isAddingBulkMedia}>Add Bulk</Button
			>
		</div>
	{/snippet}

	<Table.Root>
		<Table.Header>
			<Table.Row>
				<Table.Head>
					<div class="flex items-center gap-1 whitespace-nowrap">
						ID
						<Button variant="ghost" size="icon" class="h-6 w-6" onclick={() => handleSort('id')}>
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
						<Button variant="ghost" size="icon" class="h-6 w-6" onclick={() => handleSort('title')}>
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
						<Button
							variant="ghost"
							size="icon"
							class="h-6 w-6"
							onclick={() => handleSort('artist')}
						>
							{#if state.sortKey === 'artist'}
								{state.sortDirection === 'asc' ? '↑' : '↓'}
							{:else}
								↕
							{/if}
						</Button>
					</div>
				</Table.Head>
				<Table.Head>
					<div class="flex items-center gap-1 whitespace-nowrap">
						Rel. Year
						<Button
							variant="ghost"
							size="icon"
							class="h-6 w-6"
							onclick={() => handleSort('release_year')}
						>
							{#if state.sortKey === 'release_year'}
								{state.sortDirection === 'asc' ? '↑' : '↓'}
							{:else}
								↕
							{/if}
						</Button>
					</div>
				</Table.Head>
				<Table.Head>Qs</Table.Head>
				<Table.Head>Spotify</Table.Head>
				<Table.Head>YouTube</Table.Head>
				<Table.Head class="text-right">Actions</Table.Head>
			</Table.Row>
		</Table.Header>
		<Table.Body>
			{#if state.isAddingBulkMedia}
				<Table.Row class="bg-green-50 dark:bg-gray-800">
					<Table.Cell></Table.Cell>
					<Table.Cell colspan={6}>
						<Textarea
							bind:value={state.bulkMediaInput}
							placeholder="Paste CSV data here, one entry per line:
Title,Artist,Release Year,Spotify ID or Link,YouTube ID or Link"
							rows={6}
							class="w-full font-mono text-sm"
						/>
						<p class="text-muted-foreground mt-1 text-xs">
							Format: Title, Artist, Year, Spotify ID/Link, YouTube ID/Link
						</p>
					</Table.Cell>
					<Table.Cell class="text-right align-top">
						<div class="flex flex-col items-end gap-2">
							<Button
								variant="outline"
								size="sm"
								onclick={handleSaveBulkMedia}
								class="bg-green-100 hover:bg-green-200 dark:bg-green-700 dark:hover:bg-green-600"
								>Save Bulk</Button
							>
							<Button
								variant="outline"
								size="sm"
								class="text-red-600 hover:bg-red-50 hover:dark:bg-red-800"
								onclick={handleCancelBulkAdd}
							>
								Cancel
							</Button>
						</div>
					</Table.Cell>
				</Table.Row>
			{/if}

			{#if state.isAddingMedia}
				<Table.Row class="bg-blue-50 dark:bg-gray-800">
					<Table.Cell class="text-muted-foreground">New</Table.Cell>
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
							min="1000"
							max="3000"
							value={state.newMediaData.release_year?.toString() || ''}
							placeholder="Year"
							onChange={(value) => (state.newMediaData.release_year = parseInt(value))}
							onCommit={(value) => (state.newMediaData.release_year = parseInt(value))}
						/>
					</Table.Cell>
					<Table.Cell class="text-center">0</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.spotify_uri || ''}
							placeholder="Spotify ID or Link"
							onChange={(value) => (state.newMediaData.spotify_uri = value)}
							onCommit={(value) => (state.newMediaData.spotify_uri = value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newMediaData.youtube_id || ''}
							placeholder="YouTube ID or Link"
							onChange={(value) => (state.newMediaData.youtube_id = value)}
							onCommit={(value) => (state.newMediaData.youtube_id = value)}
						/>
					</Table.Cell>
					<Table.Cell class="text-right">
						<div class="flex justify-end gap-2">
							<Button variant="outline" size="sm" onclick={handleSaveMedia}>Save</Button>
							<Button
								variant="outline"
								size="sm"
								class="text-red-600 hover:bg-red-50 hover:dark:bg-red-800"
								onclick={handleCancelAdd}
							>
								Cancel
							</Button>
						</div>
					</Table.Cell>
				</Table.Row>
			{/if}

			{#each paginatedData as media (media.id)}
				<Table.Row class={cn('transition-colors hover:bg-gray-50 dark:hover:bg-gray-800')}>
					<Table.Cell class="font-medium">{media.id}</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={String(media.title ?? '')}
							onCommit={(value) => commitMediaField(media.id, 'title', value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={String(media.artist ?? '')}
							onCommit={(value) => commitMediaField(media.id, 'artist', value)}
						/>
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							type="number"
							min="1000"
							max="3000"
							value={String(media.release_year ?? '')}
							placeholder="Year"
							onCommit={(value) => commitMediaField(media.id, 'release_year', value)}
						/>
					</Table.Cell>
					<Table.Cell class="text-center">
						{#if getQuestionCount(media.id) > 0}
							<span
								class="inline-flex items-center justify-center rounded-full bg-blue-100 px-2 py-0.5 text-xs font-semibold text-blue-800 dark:bg-blue-900 dark:text-blue-200"
							>
								{getQuestionCount(media.id)}
							</span>
						{:else}
							<span class="text-muted-foreground text-xs">0</span>
						{/if}
					</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={String(media.spotify_uri ?? '')}
							placeholder="Spotify ID or Link"
							onCommit={(value) => commitMediaField(media.id, 'spotify_uri', value)}
						/>
						{#if media.spotify_uri}
							{@const spotifyId = parseSpotifyLink(media.spotify_uri)}
							{#if spotifyId}
								<a
									href={`https://open.spotify.com/track/${spotifyId}`}
									target="_blank"
									rel="noopener noreferrer"
									class="mt-1 block text-xs text-blue-600 hover:underline"
									title={`Open Spotify track: ${spotifyId}`}
								>
									Open Spotify
								</a>
							{/if}
						{/if}
					</Table.Cell>
					<Table.Cell>
						<div class="flex items-start gap-2">
							<div class="flex-1">
								<EditableInput
									value={String(media.youtube_id ?? '')}
									placeholder="YouTube ID or Link"
									onCommit={(value) => commitMediaField(media.id, 'youtube_id', value)}
								/>
								{#if media.youtube_id}
									{@const youtubeId = parseYoutubeLink(media.youtube_id)}
									{#if youtubeId}
										<a
											href={`https://youtube.com/watch?v=${youtubeId}`}
											target="_blank"
											rel="noopener noreferrer"
											class="mt-1 block text-xs text-blue-600 hover:underline"
											title={`Open YouTube video: ${youtubeId}`}
										>
											Open YouTube
										</a>
									{/if}
								{/if}
							</div>
							{#if media.youtube_id}
								{@const youtubeId = parseYoutubeLink(media.youtube_id)}
								{#if youtubeId}
									<img
										class="mt-1 h-9 w-16 flex-shrink-0 rounded object-cover"
										src={`https://i.ytimg.com/vi/${youtubeId}/default.jpg`}
										loading="lazy"
										alt={`YT: ${media.title}`}
									/>
								{/if}
							{/if}
						</div>
					</Table.Cell>
					<Table.Cell class="text-right">
						<div class="flex justify-end gap-2">
							<Button
								variant="outline"
								size="sm"
								class="disabled:text-muted-foreground text-red-600 hover:bg-red-100 disabled:hover:bg-transparent dark:hover:bg-red-800/50 disabled:dark:hover:bg-transparent"
								onclick={() => handleDeleteMedia(media.id)}
								disabled={isMediaUsed(media.id)}
								title={isMediaUsed(media.id)
									? `Cannot delete: Used in ${getQuestionCount(media.id)} question(s)`
									: 'Delete Media'}
							>
								Delete
							</Button>
						</div>
					</Table.Cell>
				</Table.Row>
			{/each}
			{#if paginatedData.length === 0 && !state.isAddingMedia && !state.isAddingBulkMedia}
				<Table.Row>
					<Table.Cell colspan={8} class="text-muted-foreground h-24 text-center">
						No media found matching your criteria.
					</Table.Cell>
				</Table.Row>
			{/if}
		</Table.Body>
	</Table.Root>

	<Pagination
		currentPage={state.currentPage}
		{totalPages}
		totalItems={filteredData.length}
		itemsPerPage={state.itemsPerPage}
		onPageChange={(page) => (state.currentPage = page)}
	/>
</TableContainer>
