<script lang="ts">
	import {
		Card,
		CardContent,
		CardFooter,
		CardHeader,
		CardTitle,
		CardDescription
	} from '#lib/components/ui/card/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { adminStore } from '#lib/stores/data-manager.svelte.ts';
	import { AlertCircle, Undo2, Redo2 } from '@lucide/svelte';
	import { cn } from '#lib/utils.ts';
	import { api } from '#lib/api.ts';

	const state = $state({
		password: '',
		error: null as string | null,
		isSubmitting: false,
		uploadProgress: 0
	});

	async function uploadPendingImages() {
		const pendingUploads = adminStore
			.getState()
			.characters.flatMap(({ name, _pendingImage }) =>
				_pendingImage ? [{ name, file: _pendingImage.file }] : []
			);

		if (pendingUploads.length === 0) return;

		state.uploadProgress = 0;
		const total = pendingUploads.length;
		let completed = 0;

		try {
			await Promise.all(
				pendingUploads.map(async ({ name, file }) => {
					await api.uploadCharacterImage(name, file, state.password);
					completed++;
					state.uploadProgress = Math.round((completed / total) * 100);
				})
			);
		} catch (error) {
			const message = error instanceof Error ? error.message : 'Unknown error';
			throw new Error(`Image upload failed: ${message}`, { cause: error });
		}
	}

	async function handleApplyChanges() {
		if (!state.password) {
			state.error = 'Password is required';
			return;
		}

		const baseVersion = adminStore.getBaseVersion();
		if (baseVersion === null) {
			state.error = 'No data loaded';
			return;
		}

		try {
			state.isSubmitting = true;
			state.error = null;

			// First upload pending images
			await uploadPendingImages();

			// Then send the main update
			const saved = await api.setStoredData({
				password: state.password,
				stored_data: adminStore.exportData(),
				base_version: baseVersion
			});
			// The saved data becomes the new starting point
			adminStore.setData(saved);
			state.password = '';
		} catch (e) {
			state.error = e instanceof Error ? e.message : 'Failed to apply changes';
		} finally {
			state.isSubmitting = false;
			state.uploadProgress = 0;
		}
	}

	function copyJson() {
		navigator.clipboard.writeText(JSON.stringify(adminStore.exportData(), null, 2));
	}
</script>

<Card class="w-full">
	<CardHeader>
		<div class="flex items-center justify-between">
			<div>
				<CardTitle>Changes History</CardTitle>
				<CardDescription>Review and manage your changes</CardDescription>
			</div>
			<div class="flex gap-2">
				<Button
					variant="outline"
					size="icon"
					disabled={!adminStore.canUndo()}
					onclick={() => adminStore.undo()}
				>
					<Undo2 class="h-4 w-4" />
				</Button>
				<Button
					variant="outline"
					size="icon"
					disabled={!adminStore.canRedo()}
					onclick={() => adminStore.redo()}
				>
					<Redo2 class="h-4 w-4" />
				</Button>
			</div>
		</div>
	</CardHeader>

	<CardContent class="space-y-6">
		{#if !adminStore.canUndo()}
			<div class="text-gray-500">No changes made yet</div>
		{:else}
			<div class="relative space-y-2 pl-6">
				<div class="absolute top-3 left-2.5 h-[calc(100%-24px)] w-px bg-gray-200"></div>

				{#each adminStore.getSnapshots() as snapshot, index (index)}
					{@const isCurrentStep = index === adminStore.getSnapshotIndex()}
					{@const isFutureStep = index > adminStore.getSnapshotIndex()}

					<div class={cn('relative flex gap-3 pl-6', isFutureStep && 'opacity-50')}>
						<div
							class={cn(
								'absolute top-1.5 left-0 h-3 w-3 rounded-full border-2 border-white',
								isCurrentStep ? 'bg-blue-500' : 'bg-gray-200',
								isFutureStep && 'bg-gray-100'
							)}
						></div>

						<div class="flex-1">
							<div
								class={cn(
									'rounded-lg border px-3 py-2 text-sm',
									isCurrentStep && 'border-blue-200 bg-blue-50 dark:bg-gray-800'
								)}
							>
								{snapshot.message}
							</div>
						</div>
					</div>
				{/each}
			</div>

			{#if state.error}
				<div class="flex items-center gap-2 rounded-md bg-red-50 p-3 text-red-600">
					<AlertCircle class="h-5 w-4" />
					<span>{state.error}</span>
				</div>
			{/if}

			<div>
				<Input
					type="password"
					autocomplete="off"
					placeholder="Enter password to apply changes"
					bind:value={state.password}
				/>
			</div>
		{/if}
	</CardContent>

	<CardFooter class="flex justify-between">
		<Button variant="outline" size="sm" onclick={copyJson}>Copy JSON</Button>
		<div class="flex gap-2">
			<Button
				disabled={state.isSubmitting || !state.password || !adminStore.canUndo()}
				onclick={handleApplyChanges}
			>
				{state.isSubmitting ? 'Applying...' : 'Apply Changes'}
			</Button>
		</div>
	</CardFooter>
</Card>
