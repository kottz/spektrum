<script lang="ts">
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Card, CardContent } from '#lib/components/ui/card/index.ts';
	import { gameActions } from '#lib/stores/game-actions.ts';
	import { notifications } from '#lib/stores/notification-store.svelte.ts';
	import { warn } from '#lib/utils/logger.ts';
	import { api } from '#lib/api.ts';
	import type { SetInfo } from '#lib/generated/SetInfo.ts';

	/** A server set, or every question when `id` is null. */
	type SetChoice = Omit<SetInfo, 'id'> & { id: number | null };

	let selectedSet = $state<number | null>(null);
	let isLoading = $state(false);
	let sets = $state<SetChoice[]>([]);

	async function fetchQuestionSets() {
		isLoading = true;
		try {
			const data = await api.listSets();
			sets = [
				{ id: null, name: 'All Questions', question_count: data.num_questions },
				...data.sets
			];
		} catch (err) {
			notifications.add('Failed to load question sets', 'destructive');
			warn('Failed to load question sets:', err);
			sets = [{ id: null, name: 'All Questions', question_count: 0 }];
		} finally {
			isLoading = false;
		}
	}

	async function handleCreate() {
		try {
			await gameActions.createGame(selectedSet);
		} catch (err) {
			warn('Lobby creation failed:', err);
			notifications.add('Failed to create lobby', 'destructive');
		}
	}
	fetchQuestionSets();
</script>

<div class="flex h-full flex-col">
	<ScrollArea class="min-h-0 flex-1">
		<div class="space-y-4 p-4">
			{#if isLoading}
				<div class="text-muted-foreground text-center">Loading question sets...</div>
			{:else}
				<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
					{#each sets as set (set.id)}
						<Card
							class={`hover:bg-accent/50 cursor-pointer transition-colors ${
								selectedSet === set.id ? 'border-primary! border-2' : ''
							}`}
							onclick={() => (selectedSet = set.id)}
						>
							<CardContent class="p-4">
								<h3 class="font-semibold">{set.name}</h3>
								<p class="text-muted-foreground text-sm">
									{set.question_count}
									{set.question_count === 1 ? 'question' : 'questions'}
								</p>
							</CardContent>
						</Card>
					{/each}
				</div>
			{/if}
		</div>
	</ScrollArea>
	<div class="bg-card border-t p-4">
		<div class="flex justify-end gap-2">
			<Button onclick={handleCreate} disabled={isLoading}>Create Lobby</Button>
		</div>
	</div>
</div>
