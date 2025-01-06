<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';

	let csvData: any[] = [];
	let distinctCharacters = new Set<string>();
	let showBank = false;
	let searchTerm = '';
	let currentPage = 0;
	let itemsPerPage = 10;

	let filteredData: any[];
	$: {
		filteredData = csvData.filter(
			(row) =>
				row.song?.toLowerCase().includes(searchTerm.toLowerCase()) ||
				row.correct_character?.toLowerCase().includes(searchTerm.toLowerCase())
		);
		// Reset to first page whenever search results change
		currentPage = 0;
	}

	let totalPages: number;
	$: totalPages = Math.ceil(filteredData.length / itemsPerPage);

	let paginatedData: any[];
	$: paginatedData = filteredData.slice(
		currentPage * itemsPerPage,
		(currentPage + 1) * itemsPerPage
	);

	function handleFileUpload(event: Event) {
		const input = event.target as HTMLInputElement;
		const file = input.files?.[0];

		if (file) {
			const reader = new FileReader();
			reader.onload = (e) => {
				const text = e.target?.result as string;
				parseCSV(text);
			};
			reader.readAsText(file);
		}
	}

	function parseCSV(text: string) {
		const lines = text.trim().split(/\r?\n/);
		if (!lines.length) return;

		const headers = lines[0].split(',').map((h) => h.trim());
		csvData = lines.slice(1).map((line) => {
			const values = line.split(',').map((v) => v.trim());
			const rowObj: any = {};
			headers.forEach((h, i) => {
				rowObj[h] = values[i] || '';
			});
			return rowObj;
		});

		// Add blank row
		csvData.push({
			id: '',
			difficulty: '',
			song: '',
			correct_character: '',
			other_characters: '',
			spotify_uri: '',
			youtube_id: ''
		});

		// Update distinct characters
		distinctCharacters.clear();
		csvData.forEach((row) => {
			if (row.correct_character) {
				distinctCharacters.add(row.correct_character);
			}
		});

		// Reset to first page when loading new data
		currentPage = 0;
	}

	function handleDrop(event: DragEvent, rowIndex: number) {
		event.preventDefault();
		const charName = event.dataTransfer?.getData('text/plain');
		if (!charName) return;

		const globalIndex = currentPage * itemsPerPage + rowIndex;
		const row = csvData[globalIndex];
		if (!row) return;

		const currentChars = row.other_characters.split(';').filter(Boolean);

		if (currentChars.length >= 5) return;

		currentChars.push(charName);
		row.other_characters = currentChars.join(';');
		csvData = [...csvData]; // Trigger reactivity
	}

	function removeCharacter(rowIndex: number, charIndex: number) {
		const globalIndex = currentPage * itemsPerPage + rowIndex;
		const row = csvData[globalIndex];
		if (!row) return;

		const chars = row.other_characters.split(';').filter(Boolean);

		chars.splice(charIndex, 1);
		row.other_characters = chars.join(';');
		csvData = [...csvData]; // Trigger reactivity
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

	function exportCSV() {
		const headers = Object.keys(csvData[0] || {});
		if (headers.length === 0) return;

		let csvContent = headers.join(',') + '\n';

		csvData.forEach((row) => {
			const rowContent = headers.map((header) => row[header] || '').join(',');
			csvContent += rowContent + '\n';
		});

		const blob = new Blob([csvContent], { type: 'text/csv' });
		const url = window.URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = 'exported_data.csv';
		a.click();
		window.URL.revokeObjectURL(url);
	}
</script>

<div class="w-full">
	<div class="flex items-center gap-4 py-4">
		<Input type="file" accept=".csv" on:change={handleFileUpload} class="max-w-sm" />
		<Input type="text" placeholder="Search..." bind:value={searchTerm} class="max-w-sm" />
		<Button variant="outline" on:click={() => (showBank = !showBank)}>Toggle Character Bank</Button>
		<Button variant="outline" on:click={exportCSV}>Export CSV</Button>
	</div>

	<div class="rounded-md border">
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>ID</Table.Head>
					<Table.Head>Difficulty</Table.Head>
					<Table.Head>Song</Table.Head>
					<Table.Head>Correct Character</Table.Head>
					<Table.Head>Other Characters</Table.Head>
					<Table.Head>Spotify URI</Table.Head>
					<Table.Head>YouTube ID</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each paginatedData as row, rowIndex}
					<Table.Row>
						<Table.Cell>{row.id}</Table.Cell>
						<Table.Cell>{row.difficulty}</Table.Cell>
						<Table.Cell>{row.song}</Table.Cell>
						<Table.Cell>
							{#if row.correct_character}
								<div class="flex flex-col items-center gap-2">
									<img
										src={`/img/${row.correct_character}.avif`}
										alt={row.correct_character}
										class="h-12 w-12 rounded"
									/>
									<span class="text-sm">{row.correct_character}</span>
								</div>
							{/if}
						</Table.Cell>
						<Table.Cell>
							<div
								class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
								on:dragover|preventDefault
								on:drop={(e) => handleDrop(e, rowIndex)}
							>
								{#each row.other_characters.split(';').filter(Boolean) as char, charIndex}
									<div class="group relative">
										<img src={`/img/${char}.avif`} alt={char} class="h-12 w-12 rounded" />
										<button
											class="absolute -right-2 -top-2 hidden h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white group-hover:flex"
											on:click={() => removeCharacter(rowIndex, charIndex)}>×</button
										>
										<span class="mt-1 text-center text-xs">{char}</span>
									</div>
								{/each}
							</div>
						</Table.Cell>
						<Table.Cell>{row.spotify_uri}</Table.Cell>
						<Table.Cell>{row.youtube_id}</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	</div>

	<div class="flex items-center justify-between py-4">
		<div class="text-sm text-muted-foreground">
			Showing {currentPage * itemsPerPage + 1} to {Math.min(
				(currentPage + 1) * itemsPerPage,
				filteredData.length
			)} of {filteredData.length} entries
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

{#if showBank}
	<div class="fixed right-0 top-16 h-[calc(100vh-4rem)] w-96 border-l bg-white shadow-lg">
		<!-- Fixed header -->
		<div class="sticky top-0 border-b bg-white p-4">
			<div class="flex items-center justify-between">
				<h2 class="text-lg font-semibold">Character Bank</h2>
				<Button variant="outline" on:click={() => (showBank = false)}>Close</Button>
			</div>
		</div>
		<!-- Scrollable content -->
		<div class="h-[calc(100%-4rem)] overflow-y-auto p-4">
			<div class="grid grid-cols-4 gap-4">
				{#each Array.from(distinctCharacters) as char}
					<div
						class="cursor-grab transition-transform hover:scale-105"
						draggable="true"
						on:dragstart={(e) => {
							e.dataTransfer?.setData('text/plain', char);
							e.currentTarget.classList.add('dragging');
						}}
						on:dragend={(e) => {
							e.currentTarget.classList.remove('dragging');
						}}
					>
						<img src={`/img/${char}.avif`} alt={char} class="w-full rounded-lg" />
						<div class="mt-1 text-center text-sm">{char}</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.dragging {
		opacity: 0.5;
	}
</style>
