<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/admin-data';
	import type { Media } from '$lib/types';

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

	// Get associated questions count
	function getQuestionCount(mediaId: number): number {
		return $adminStore.questions.filter((q) => q.media_id === mediaId).length;
	}

	function nextPage() {
		if (currentPage < totalPages - 1) {
			currentPage++;
		}
	}

	function previousPage() {
		if (currentPage > 0) {
			currentPage--;
		}
	}

	function handleAddMedia() {
		// TODO: Implement add media functionality
		console.log('Add media clicked');
	}

	function handleEditMedia(media: Media) {
		// TODO: Implement edit media functionality
		console.log('Edit media:', media);
	}

	function handleDeleteMedia(mediaId: number) {
		// TODO: Implement delete media functionality
		console.log('Delete media:', mediaId);
	}

	function formatSpotifyUri(uri: string | null): string {
		if (!uri) return '';
		// Extract just the ID if it's a full URI
		return uri.includes('spotify:') ? uri.split(':').pop() || '' : uri;
	}
</script>

<div class="w-full">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-4">
			<Input type="text" placeholder="Search media..." bind:value={searchTerm} class="max-w-sm" />
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
				{#each paginatedData as media}
					<Table.Row>
						<Table.Cell>{media.id}</Table.Cell>
						<Table.Cell class="font-medium">{media.title}</Table.Cell>
						<Table.Cell>{media.artist}</Table.Cell>
						<Table.Cell>{media.release_year || 'N/A'}</Table.Cell>
						<Table.Cell>
							<span
								class="inline-flex rounded-full bg-blue-100 px-2 py-1 text-xs font-semibold text-blue-800"
							>
								{getQuestionCount(media.id)}
							</span>
						</Table.Cell>
						<Table.Cell>
							{#if media.spotify_uri}
								<a
									href={`https://open.spotify.com/track/${formatSpotifyUri(media.spotify_uri)}`}
									target="_blank"
									rel="noopener noreferrer"
									class="text-blue-600 hover:underline"
								>
									Link
								</a>
							{:else}
								-
							{/if}
						</Table.Cell>
						<Table.Cell>
							{#if media.youtube_id}
								<div class="flex items-center gap-2">
									<a
										href={`https://youtube.com/watch?v=${media.youtube_id}`}
										target="_blank"
										rel="noopener noreferrer"
										class="text-blue-600 hover:underline"
									>
										Link
									</a>
									<img
										src={`https://i.ytimg.com/vi_webp/${media.youtube_id}/default.webp`}
										alt={`YouTube title for ${media.title}`}
										class="mb-0 mt-0 h-12 w-12 object-contain"
									/>
								</div>
							{:else}
								-
							{/if}
						</Table.Cell>
						<Table.Cell class="text-right">
							<div class="flex justify-end gap-2">
								<Button variant="outline" size="sm" on:click={() => handleEditMedia(media)}>
									Edit
								</Button>
								<Button
									variant="outline"
									size="sm"
									class="text-red-600 hover:bg-red-50"
									on:click={() => handleDeleteMedia(media.id)}
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
