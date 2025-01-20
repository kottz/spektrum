<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/admin-data';
	import { AlertCircle } from 'lucide-svelte';
	import { PUBLIC_DEV_ADMIN_PASSWORD } from '$env/static/public';

	let password = '';
	let error: string | null = null;
	let isSubmitting = false;
	let isOpen = false;

	function copyJson() {
		console.log('final state', adminStore.getFinalState());
		const jsonData = JSON.stringify(adminStore.getFinalState(), null, 2);
		navigator.clipboard.writeText(jsonData);
	}

	async function handleApplyChanges() {
		if (!password) {
			error = 'Password is required';
			return;
		}

		try {
			isSubmitting = true;
			error = null;

			const response = await fetch('http://localhost:8765/api/update-questions', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					password: PUBLIC_DEV_ADMIN_PASSWORD,
					stored_data: adminStore.getFinalState()
				})
			});

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			// Reset changes after successful update
			adminStore.setData(await response.json());
			// Only close on success
			isOpen = false;
			// Reset password
			password = '';
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to apply changes';
		} finally {
			isSubmitting = false;
		}
	}

	function getEntityTitle(entityType: string, id: number): string {
		switch (entityType) {
			case 'questions':
				return $adminStore.questions.find((q) => q.id === id)?.question_text || `Question ${id}`;
			case 'media':
				return $adminStore.media.find((m) => m.id === id)?.title || `Media ${id}`;
			case 'options':
				return $adminStore.options.find((o) => o.id === id)?.option_text || `Option ${id}`;
			case 'sets':
				return $adminStore.sets.find((s) => s.id === id)?.name || `Set ${id}`;
			default:
				return `Unknown ${id}`;
		}
	}
</script>

<AlertDialog.Root bind:open={isOpen}>
	<AlertDialog.Trigger asChild let:builder>
		<Button builders={[builder]} variant="outline" class="gap-2">
			Review Changes
			<span class="rounded-full bg-red-100 px-2 py-0.5 text-xs font-semibold text-red-600">
				{$adminStore.pendingChanges.length}
			</span>
		</Button>
	</AlertDialog.Trigger>
	<AlertDialog.Portal>
		<AlertDialog.Overlay />
		<AlertDialog.Content>
			<AlertDialog.Header>
				<AlertDialog.Title>Review Changes</AlertDialog.Title>
				<AlertDialog.Description>
					Review and apply your changes to the database
				</AlertDialog.Description>
			</AlertDialog.Header>
			<div class="space-y-6 py-4">
				{#if $adminStore.pendingChanges.length === 0}
					<div class="text-gray-500">No pending changes</div>
				{:else}
					<!-- Added Items -->
					{#if $adminStore.pendingChanges.some((change) => change.type === 'added')}
						<div class="space-y-2">
							<h3 class="font-medium text-green-600">Added</h3>
							{#each $adminStore.pendingChanges.filter((c) => c.type === 'added') as change}
								<div class="flex items-center gap-2 rounded bg-green-50 px-3 py-2">
									<span class="font-medium">{change.entityType}:</span>
									<span>{getEntityTitle(change.entityType, change.id)}</span>
								</div>
							{/each}
						</div>
					{/if}

					<!-- Deleted Items -->
					{#if $adminStore.pendingChanges.some((change) => change.type === 'deleted')}
						<div class="space-y-2">
							<h3 class="font-medium text-red-600">Deleted</h3>
							{#each $adminStore.pendingChanges.filter((c) => c.type === 'deleted') as change}
								<div class="flex items-center gap-2 rounded bg-red-50 px-3 py-2">
									<span class="font-medium">{change.entityType}:</span>
									<span>{getEntityTitle(change.entityType, change.id)}</span>
								</div>
							{/each}
						</div>
					{/if}

					{#if error}
						<div class="flex items-center gap-2 rounded-md bg-red-50 p-3 text-red-600">
							<AlertCircle class="h-5 w-5" />
							<span>{error}</span>
						</div>
					{/if}

					<div>
						<Input
							type="password"
							placeholder="Enter password to apply changes"
							bind:value={password}
						/>
					</div>
				{/if}
			</div>
			<AlertDialog.Footer class="flex justify-between">
				<Button variant="outline" size="sm" on:click={copyJson}>Copy JSON</Button>
				<div class="flex gap-2">
					<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
					<Button disabled={isSubmitting || !password} on:click={handleApplyChanges}>
						{isSubmitting ? 'Applying...' : 'Apply Changes'}
					</Button>
				</div>
			</AlertDialog.Footer>
		</AlertDialog.Content>
	</AlertDialog.Portal>
</AlertDialog.Root>
