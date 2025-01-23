<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { adminStore } from '$lib/stores/data-manager.svelte';

	const state = $state({
		isDragging: false
	});

	async function validateFile(file: File): Promise<boolean> {
		if (!file.type.includes('avif')) {
			alert('Only AVIF images are allowed');
			return false;
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
				alert('Image must be exactly 300x300 pixels');
				return false;
			}

			return true;
		} catch (error) {
			alert('Error validating image');
			return false;
		}
	}

	async function handleFileSelect(event: Event) {
		const input = event.target as HTMLInputElement;
		const files = input.files;

		if (files) {
			for (const file of files) {
				const isValid = await validateFile(file);
				if (isValid) {
					const fileName = file.name.split('.')[0];
					adminStore.addPendingCharacter(fileName, file);
				}
			}
		}
		// Reset input
		input.value = '';
	}

	function handleDrop(event: DragEvent) {
		event.preventDefault();
		state.isDragging = false;

		const files = event.dataTransfer?.files;
		if (files) {
			for (const file of files) {
				validateFile(file).then((isValid) => {
					if (isValid) {
						const fileName = file.name.split('.')[0];
						adminStore.addPendingCharacter(fileName, file);
					}
				});
			}
		}
	}

	function handleDragOver(event: DragEvent) {
		event.preventDefault();
		state.isDragging = true;
	}

	function handleDragLeave(event: DragEvent) {
		event.preventDefault();
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

		<!-- Upload Area -->
		<div
			class="my-4 flex flex-col items-center justify-center"
			ondragover={handleDragOver}
			ondragleave={handleDragLeave}
			ondrop={handleDrop}
			aria-selected="false"
			tabindex="0"
			role="option"
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
				<div class="text-center">
					<p class="text-sm text-gray-600">
						{state.isDragging ? 'Drop files here' : 'Click or drag files to upload'}
					</p>
				</div>
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
				<Button variant="outline">Done</Button>
			</Dialog.Close>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
