<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { adminStore } from '$lib/stores/data-manager.svelte';

	const state = $state({
		isDragging: false,
		uploadedFiles: [] as Array<{
			id: string;
			fileName: string;
			file: File;
			status: 'pending' | 'success' | 'error';
			error?: string;
		}>
	});

	async function validateFile(file: File): Promise<{ valid: boolean; error?: string }> {
		if (!file.type.includes('avif')) {
			return { valid: false, error: 'Only AVIF images are allowed' };
		}

		try {
			const img = new Image();
			const imgLoaded = new Promise((resolve, reject) => {
				img.onload = () => resolve(true);
				img.onerror = () => reject(new Error('Failed to load image'));
			});
			img.src = URL.createObjectURL(file);
			await imgLoaded;

			if (img.width !== 300 || img.height !== 300) {
				return { valid: false, error: 'Image must be exactly 300x300 pixels' };
			}

			return { valid: true };
		} catch (error) {
			return { valid: false, error: 'Error validating image' };
		}
	}

	async function handleFileUpload(files: FileList) {
		const newFiles = Array.from(files).map((file) => ({
			id: crypto.randomUUID(),
			fileName: file.name.split('.')[0],
			file,
			status: 'pending' as const,
			error: ''
		}));

		state.uploadedFiles = [...state.uploadedFiles, ...newFiles];

		for (const newFile of newFiles) {
			try {
				const result = await validateFile(newFile.file);
				if (result.valid) {
					adminStore.addPendingCharacter(newFile.fileName, newFile.file);
					const index = state.uploadedFiles.findIndex((f) => f.id === newFile.id);
					if (index !== -1) {
						state.uploadedFiles[index].status = 'success';
					}
				} else {
					const index = state.uploadedFiles.findIndex((f) => f.id === newFile.id);
					if (index !== -1) {
						state.uploadedFiles[index].status = 'error';
						state.uploadedFiles[index].error = result.error;
					}
				}
			} catch (error) {
				const index = state.uploadedFiles.findIndex((f) => f.id === newFile.id);
				if (index !== -1) {
					state.uploadedFiles[index].status = 'error';
					state.uploadedFiles[index].error = 'Error validating file';
				}
			}
		}
	}

	async function handleFileSelect(event: Event) {
		const input = event.target as HTMLInputElement;
		if (input.files) await handleFileUpload(input.files);
		input.value = '';
	}

	async function handleDrop(event: DragEvent) {
		event.preventDefault();
		state.isDragging = false;
		if (event.dataTransfer?.files) await handleFileUpload(event.dataTransfer.files);
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		state.isDragging = true;
	}

	function handleDragLeave(event: DragEvent) {
		event.preventDefault();
		state.isDragging = false;
	}
</script>

<Dialog.Root>
	<Dialog.Trigger>
		<Button variant="outline" size="sm">Upload</Button>
	</Dialog.Trigger>

	<Dialog.Content class="sm:max-w-[425px]">
		<Dialog.Header>
			<Dialog.Title>Upload Characters</Dialog.Title>
			<Dialog.Description>
				Drag and drop AVIF images or click to select files. Images must be 300x300 pixels.
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
                    flex h-48 w-full cursor-pointer flex-col
                    items-center justify-center gap-4 rounded-lg border-2
                    border-dashed transition-colors
                    ${state.isDragging ? 'border-primary bg-primary/10' : 'border-gray-300'}
                `}
				onclick={() => document.getElementById('file-upload')?.click()}
			>
				{#if state.isDragging}
					<p class="text-sm text-gray-600">Drop files here</p>
				{:else if state.uploadedFiles.length === 0}
					<p class="text-sm text-gray-600">Click or drag files to upload</p>
				{:else}
					<div class="max-h-40 w-full space-y-2 overflow-y-auto px-4">
						{#each state.uploadedFiles as file (file.id)}
							<div
								class="flex items-center justify-between rounded-lg border p-2"
								class:border-green-500={file.status === 'success'}
								class:border-red-500={file.status === 'error'}
								class:border-gray-300={file.status === 'pending'}
							>
								<span class="truncate text-sm">{file.fileName}</span>
								{#if file.status === 'error'}
									<span class="text-sm text-red-500">{file.error}</span>
								{:else if file.status === 'pending'}
									<span class="text-sm text-gray-500">Validating...</span>
								{:else}
									<span class="text-sm text-green-500">✓</span>
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
						state.uploadedFiles = [];
					}}
				>
					Done
				</Button>
			</Dialog.Close>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
