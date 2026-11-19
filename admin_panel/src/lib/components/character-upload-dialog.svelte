<script lang="ts">
	import * as Dialog from '#lib/components/ui/dialog/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { adminStore } from '#lib/stores/data-manager.svelte.ts';

	/**
	 * Per-file lifecycle: pending → added | invalid | conflict; conflict → (Overwrite) → added.
	 * A conflict is a valid image whose name already exists.
	 */
	type FileStatus =
		| { kind: 'pending' }
		| { kind: 'added' }
		| { kind: 'invalid'; reason: string }
		| { kind: 'conflict' };

	interface UploadEntry {
		id: string;
		name: string;
		file: File;
		dataUrl: string;
		status: FileStatus;
	}

	const state = $state({
		isDragging: false,
		files: [] as UploadEntry[]
	});

	/** Mirrors the server's rule for character names (they become file names). */
	const CHARACTER_NAME = /^[A-Za-z0-9_-]+$/;

	function setStatus(id: string, status: FileStatus) {
		state.files = state.files.map((f) => (f.id === id ? { ...f, status } : f));
	}

	/** Returns why the image is unusable, or null when it is a valid character image. */
	async function invalidReason(file: File, name: string): Promise<string | null> {
		if (!file.type.includes('avif')) return 'Only AVIF images are allowed';
		if (!CHARACTER_NAME.test(name)) {
			return 'File name may only contain letters, digits, _ and -';
		}
		const url = URL.createObjectURL(file);
		try {
			const { width, height } = await new Promise<{ width: number; height: number }>(
				(resolve, reject) => {
					const img = new Image();
					img.onload = () => resolve({ width: img.width, height: img.height });
					img.onerror = reject;
					img.src = url;
				}
			);
			if (width !== 300 || height !== 300) return 'Image must be exactly 300x300 pixels';
			return null;
		} catch {
			return 'Failed to load image';
		} finally {
			URL.revokeObjectURL(url);
		}
	}

	function readFile(file: File): Promise<string> {
		return new Promise((resolve, reject) => {
			const reader = new FileReader();
			reader.onload = () => resolve(reader.result as string);
			reader.onerror = () => reject(reader.error ?? new Error('Failed to read file'));
			reader.readAsDataURL(file);
		});
	}

	function findCharacter(name: string) {
		return adminStore.getState().characters.find((c) => c.name === name);
	}

	async function processFile(file: File) {
		const id = crypto.randomUUID();
		const name = file.name.replace(/\.avif$/i, '');
		let dataUrl: string;
		try {
			dataUrl = await readFile(file);
		} catch (error) {
			const reason = error instanceof Error ? error.message : 'Failed to read file';
			state.files = [
				{ id, name, file, dataUrl: '', status: { kind: 'invalid', reason } },
				...state.files
			];
			return;
		}
		state.files = [{ id, name, file, dataUrl, status: { kind: 'pending' } }, ...state.files];

		const reason = await invalidReason(file, name);
		if (reason) {
			setStatus(id, { kind: 'invalid', reason });
			return;
		}
		if (findCharacter(name)) {
			setStatus(id, { kind: 'conflict' });
			return;
		}

		adminStore.addEntity('characters', {
			name,
			image_url: `img/${name}.avif`,
			_pendingImage: { dataUrl, file }
		});
		setStatus(id, { kind: 'added' });
	}

	function handleOverwrite(entry: UploadEntry) {
		const existing = findCharacter(entry.name);
		if (!existing) {
			setStatus(entry.id, { kind: 'invalid', reason: 'Character no longer exists' });
			return;
		}
		adminStore.modifyEntity('characters', existing.id, {
			_pendingImage: { dataUrl: entry.dataUrl, file: entry.file }
		});
		setStatus(entry.id, { kind: 'added' });
	}

	let fileInput: HTMLInputElement | undefined;

	function openFilePicker() {
		fileInput?.click();
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
			role="region"
			aria-label="Character image upload"
		>
			<div
				class={`
					flex h-48 w-full cursor-pointer flex-col items-center justify-center gap-4
					rounded-lg border-2 border-dashed transition-colors
					${state.isDragging ? 'border-primary bg-primary/10' : 'border-gray-300'}
				`}
				role="button"
				tabindex="0"
				onclick={openFilePicker}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault();
						openFilePicker();
					}
				}}
			>
				{#if state.files.length === 0}
					<p class="text-sm text-gray-600">Drag files here or click to upload</p>
				{:else}
					<div class="max-h-40 w-full space-y-2 overflow-y-auto px-4">
						{#each state.files as file (file.id)}
							<div
								class="group flex items-center justify-between rounded-lg border p-2"
								class:border-green-500={file.status.kind === 'added'}
								class:border-red-500={file.status.kind === 'invalid' ||
									file.status.kind === 'conflict'}
								class:border-blue-500={file.status.kind === 'pending'}
							>
								<div class="flex items-center gap-2">
									{#if file.dataUrl}
										<img src={file.dataUrl} alt={file.name} class="h-8 w-8 rounded object-cover" />
									{/if}
									<span class="text-sm font-medium">{file.name}</span>
								</div>

								{#if file.status.kind === 'invalid'}
									<span class="text-sm text-red-500">{file.status.reason}</span>
								{:else if file.status.kind === 'conflict'}
									<div class="flex items-center gap-2">
										<span class="text-sm text-red-500">Character "{file.name}" exists</span>
										<Button
											variant="destructive"
											size="sm"
											onclick={(e) => {
												e.stopPropagation();
												e.preventDefault();
												handleOverwrite(file);
											}}
										>
											Overwrite
										</Button>
									</div>
								{:else if file.status.kind === 'pending'}
									<div class="flex items-center gap-2">
										<div class="h-2 w-2 animate-pulse rounded-full bg-blue-500"></div>
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
				bind:this={fileInput}
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
					onclick={() => {
						state.files = state.files.filter((f) => f.status.kind !== 'added');
					}}
				>
					Close
				</Button>
			</Dialog.Close>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
