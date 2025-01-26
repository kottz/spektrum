<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { adminStore } from '$lib/stores/data-manager.svelte';

	type FileStatus = 'pending' | 'success' | 'error' | 'overwrite';

	const state = $state({
		isDragging: false,
		files: [] as Array<{
			id: string;
			name: string;
			file: File;
			status: FileStatus;
			error?: string;
			dataUrl?: string;
		}>
	});

	async function validateFile(file: File) {
		// Basic validation
		if (!file.type.includes('avif')) {
			return { valid: false, error: 'Only AVIF images are allowed' };
		}

		// Dimension validation
		const dimensionPromise = new Promise<{ width: number; height: number }>((resolve, reject) => {
			const img = new Image();
			img.onload = () => resolve({ width: img.width, height: img.height });
			img.onerror = reject;
			img.src = URL.createObjectURL(file);
		});

		try {
			const { width, height } = await dimensionPromise;
			if (width !== 300 || height !== 300) {
				return { valid: false, error: 'Image must be exactly 300x300 pixels' };
			}
		} catch {
			return { valid: false, error: 'Failed to load image' };
		}

		// Name check
		const fileName = file.name.replace(/\.avif$/i, '');
		const exists = adminStore.getState().characters.some((c) => c.name === fileName);

		return {
			valid: !exists,
			error: exists ? `Character "${fileName}" exists` : undefined,
			exists
		};
	}

	async function readFile(file: File): Promise<string> {
		return new Promise((resolve) => {
			const reader = new FileReader();
			reader.onload = () => resolve(reader.result as string);
			reader.readAsDataURL(file);
		});
	}

	function generateUniqueId(existingCharacters: any[]) {
		const maxId = Math.max(...existingCharacters.map((c) => c.id), 0);
		return maxId + 1;
	}

	async function processFile(file: File) {
		const id = crypto.randomUUID();
		const fileName = file.name.replace(/\.avif$/i, '');
		const dataUrl = await readFile(file);

		state.files = [{ id, name: fileName, file, status: 'pending', dataUrl }, ...state.files];

		try {
			const validation = await validateFile(file);
			const existing = adminStore.getState().characters.find((c) => c.name === fileName);

			if (validation.exists && existing) {
				// If file exists but user hasn't chosen to overwrite yet
				state.files = state.files.map((f) =>
					f.id === id ? { ...f, status: 'error', error: validation.error } : f
				);
				return;
			}

			adminStore.startBatch();

			if (existing) {
				// Update existing character
				adminStore.modifyEntity('characters', existing.id, {
					_pendingImage: { dataUrl, file }
				});
			} else {
				// Create new character
				const newId = generateUniqueId(adminStore.getState().characters);
				adminStore.addEntity('characters', {
					id: newId,
					name: fileName,
					image_url: `img/${fileName}.avif`,
					_pendingImage: { dataUrl, file }
				});
			}

			adminStore.commitBatch();
			state.files = state.files.map((f) => (f.id === id ? { ...f, status: 'success' } : f));
		} catch (error) {
			adminStore.cancelBatch();
			state.files = state.files.map((f) =>
				f.id === id ? { ...f, status: 'error', error: error.message } : f
			);
		}
	}

	async function handleOverwrite(fileId: string) {
		const file = state.files.find((f) => f.id === fileId);
		if (!file) return;

		try {
			adminStore.startBatch();
			const existing = adminStore.getState().characters.find((c) => c.name === file.name);

			if (existing) {
				// Update existing character with new image
				adminStore.modifyEntity('characters', existing.id, {
					_pendingImage: { dataUrl: file.dataUrl!, file: file.file }
				});
			}

			adminStore.commitBatch();
			state.files = state.files.map((f) =>
				f.id === fileId ? { ...f, status: 'success', error: undefined } : f
			);
		} catch (error) {
			adminStore.cancelBatch();
			state.files = state.files.map((f) =>
				f.id === fileId ? { ...f, status: 'error', error: error.message } : f
			);
		}
	}

	async function handleFileSelect(event: Event) {
		const input = event.target as HTMLInputElement;
		if (!input.files) return;

		await Promise.all(Array.from(input.files).map(processFile));
		input.value = '';
	}

	async function handleDrop(event: DragEvent) {
		event.preventDefault();
		state.isDragging = false;

		if (event.dataTransfer?.files) {
			await Promise.all(Array.from(event.dataTransfer.files).map(processFile));
		}
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		event.stopPropagation();
		state.isDragging = true;
	}

	function handleDragLeave(event: DragEvent) {
		event.preventDefault();
		state.isDragging = false;
	}
</script>

<Dialog.Root>
	<Dialog.Trigger>
		<Button variant="outline" size="sm">Upload Characters</Button>
	</Dialog.Trigger>

	<Dialog.Content class="sm:max-w-[425px]">
		<Dialog.Header>
			<Dialog.Title>Upload Characters</Dialog.Title>
			<Dialog.Description>
				Upload 300x300 AVIF images. File name becomes character name.
			</Dialog.Description>
		</Dialog.Header>

		<div
			class="my-4 flex flex-col items-center justify-center"
			ondragover={handleDragOver}
			ondragleave={handleDragLeave}
			ondrop={handleDrop}
			tabindex="0"
		>
			<div
				class={`
					flex h-48 w-full cursor-pointer flex-col items-center justify-center gap-4
					rounded-lg border-2 border-dashed transition-colors
					${state.isDragging ? 'border-primary bg-primary/10' : 'border-gray-300'}
				`}
				onclick={() => document.getElementById('file-upload')?.click()}
			>
				{#if state.files.length === 0}
					<p class="text-sm text-gray-600">Drag files here or click to upload</p>
				{:else}
					<div class="max-h-40 w-full space-y-2 overflow-y-auto px-4">
						{#each state.files as file (file.id)}
							<div
								class="group flex items-center justify-between rounded-lg border p-2"
								class:border-green-500={file.status === 'success'}
								class:border-red-500={file.status === 'error'}
								class:border-blue-500={file.status === 'pending'}
							>
								<div class="flex items-center gap-2">
									{#if file.dataUrl}
										<img src={file.dataUrl} alt={file.name} class="h-8 w-8 rounded object-cover" />
									{/if}
									<span class="text-sm font-medium">{file.name}</span>
								</div>

								{#if file.status === 'error'}
									<div class="flex items-center gap-2">
										<span class="text-sm text-red-500">{file.error}</span>
										{#if file.error?.includes('exists')}
											<Button
												variant="destructive"
												size="sm"
												onclick={(e) => {
													e.stopPropagation();
													e.preventDefault();
													handleOverwrite(file.id);
												}}
											>
												Overwrite
											</Button>
										{/if}
									</div>
								{:else if file.status === 'pending'}
									<div class="flex items-center gap-2">
										<div class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
										<span class="text-sm text-gray-500">Processing...</span>
									</div>
								{:else}
									<div class="text-green-500">✓</div>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<input
				id="file-upload"
				type="file"
				accept="image/avif"
				multiple
				class="hidden"
				onchange={handleFileSelect}
			/>
		</div>

		<Dialog.Footer>
			<Dialog.Close>
				<Button
					variant="outline"
					on:click={() => {
						state.files = state.files.filter((f) => f.status !== 'success');
					}}
				>
					Close
				</Button>
			</Dialog.Close>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
