<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/admin-data';
	import type { Media } from '$lib/types';
	import ChangesReview from '$lib/components/changes-review.svelte';
	import { cn } from '$lib/utils';

	// Pagination state
	let currentPage = 0;
	let itemsPerPage = 10;
	let searchTerm = '';

	// Filtered and paginated data
	$: filteredData = $adminStore.media.filter((media) => {
		currentPage = 0; // Reset to first page when we change search term
		return (
			media.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
			media.artist.toLowerCase().includes(searchTerm.toLowerCase()) ||
			media.id.toString().includes(searchTerm)
		);
	});

	$: totalPages = Math.ceil(filteredData.length / itemsPerPage);
	$: paginatedData = filteredData.slice(
		currentPage * itemsPerPage,
		(currentPage + 1) * itemsPerPage
	);

	// Add media state
	let isAddingMedia = false;
	let newMediaData: Partial<Media> = {
		title: '',
		artist: '',
		release_year: new Date().getFullYear(),
		spotify_uri: '',
		youtube_id: ''
	};

	// Delete tracking
	let mediaMarkedForDeletion = new Set<number>();

	// Media field handlers
	function handleMediaFieldChange(id: number, field: keyof Media, value: string | number) {
		adminStore.modifyEntity('media', id, { [field]: value });
	}

	function handleAddMedia() {
		const maxId = Math.max(0, ...$adminStore.media.map((m) => m.id));
		newMediaData = {
			id: maxId + 1,
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		};
		isAddingMedia = true;
	}

	function handleSaveMedia() {
		if (newMediaData.title && newMediaData.artist) {
			adminStore.addEntity('media', newMediaData as Media);
			isAddingMedia = false;
			newMediaData = {
				title: '',
				artist: '',
				release_year: new Date().getFullYear(),
				spotify_uri: '',
				youtube_id: ''
			};
		}
	}

	function handleCancelAdd() {
		isAddingMedia = false;
		newMediaData = {
			title: '',
			artist: '',
			release_year: new Date().getFullYear(),
			spotify_uri: '',
			youtube_id: ''
		};
	}

	function handleDeleteMedia(mediaId: number) {
		if (mediaMarkedForDeletion.has(mediaId)) {
			adminStore.undoDelete('media', mediaId);
			mediaMarkedForDeletion.delete(mediaId);
		} else {
			adminStore.markForDeletion('media', mediaId);
			mediaMarkedForDeletion.add(mediaId);
		}
		mediaMarkedForDeletion = mediaMarkedForDeletion;
	}

	function getQuestionCount(mediaId: number): number {
		return $adminStore.questions.filter((q) => q.media_id === mediaId).length;
	}

	function formatSpotifyUri(uri: string | null): string {
		if (!uri) return '';
		return uri.includes('spotify:') ? uri.split(':').pop() || '' : uri;
	}

	function nextPage() {
		if (currentPage < totalPages - 1) currentPage++;
	}

	function previousPage() {
		if (currentPage > 0) currentPage--;
	}
</script>

<div class="w-full">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-4">
			<Input type="text" placeholder="Search media..." bind:value={searchTerm} class="max-w-sm" />
		</div>
		<div class="flex gap-2">
			<Button on:click={handleAddMedia}>Add Media</Button>
			{#if $adminStore.pendingChanges.length > 0}
				<ChangesReview />
			{/if}
		</div>
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
				{#if isAddingMedia}
					<Table.Row class="bg-blue-50">
						<Table.Cell>{newMediaData.id}</Table.Cell>
						<Table.Cell>
							<Input
								bind:value={newMediaData.title}
								placeholder="Title"
								on:input={(e) => (newMediaData.title = e.target.value)}
							/>
						</Table.Cell>
						<Table.Cell>
							<Input
								bind:value={newMediaData.artist}
								placeholder="Artist"
								on:input={(e) => (newMediaData.artist = e.target.value)}
							/>
						</Table.Cell>
						<Table.Cell>
							<Input
								type="number"
								bind:value={newMediaData.release_year}
								on:input={(e) => (newMediaData.release_year = parseInt(e.target.value))}
							/>
						</Table.Cell>
						<Table.Cell>0</Table.Cell>
						<Table.Cell>
							<Input
								bind:value={newMediaData.spotify_uri}
								placeholder="Spotify URI"
								on:input={(e) => (newMediaData.spotify_uri = e.target.value)}
							/>
						</Table.Cell>
						<Table.Cell>
							<Input
								bind:value={newMediaData.youtube_id}
								placeholder="YouTube ID"
								on:input={(e) => (newMediaData.youtube_id = e.target.value)}
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

				{#each paginatedData as media (media.id)}
					<Table.Row
						class={cn(
							'transition-colors',
							mediaMarkedForDeletion.has(media.id)
								? '!hover:bg-red-100 bg-red-100 hover:bg-red-100'
								: 'hover:bg-gray-50'
						)}
					>
						<Table.Cell>{media.id}</Table.Cell>
						<Table.Cell>
							<Input
								value={media.title}
								on:input={(e) => handleMediaFieldChange(media.id, 'title', e.target.value)}
							/>
						</Table.Cell>
						<Table.Cell>
							<Input
								value={media.artist}
								on:input={(e) => handleMediaFieldChange(media.id, 'artist', e.target.value)}
							/>
						</Table.Cell>
						<Table.Cell>
							<Input
								type="number"
								value={media.release_year || ''}
								on:input={(e) =>
									handleMediaFieldChange(media.id, 'release_year', parseInt(e.target.value))}
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
								value={media.spotify_uri || ''}
								placeholder="Spotify URI"
								on:input={(e) => handleMediaFieldChange(media.id, 'spotify_uri', e.target.value)}
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
							<Input
								value={media.youtube_id || ''}
								placeholder="YouTube ID"
								on:input={(e) => handleMediaFieldChange(media.id, 'youtube_id', e.target.value)}
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
						</Table.Cell>
						<Table.Cell class="text-right">
							<div class="flex justify-end gap-2">
								<Button
									variant="outline"
									size="sm"
									class={mediaMarkedForDeletion.has(media.id)
										? 'text-green-600 hover:bg-green-50'
										: 'text-red-600 hover:bg-red-50'}
									on:click={() => handleDeleteMedia(media.id)}
								>
									{mediaMarkedForDeletion.has(media.id) ? 'Undo' : 'Delete'}
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
			Showing {currentPage * itemsPerPage + 1} to {Math.min(
				(currentPage + 1) * itemsPerPage,
				filteredData.length
			)} of {filteredData.length} media entries
		</div>
		<div class="flex gap-2">
			<Button variant="outline" size="sm" on:click={previousPage} disabled={currentPage === 0}>
				Previous
			</Button>
			<Button
				variant="outline"
				size="sm"
				on:click={nextPage}
				disabled={currentPage >= totalPages - 1}
			>
				Next
			</Button>
		</div>
	</div>
</div>
