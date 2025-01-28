<script lang="ts">
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { gameActions } from '$lib/stores/game-actions';
	import { notifications } from '$lib/stores/notification-store';
	import { warn } from '$lib/utils/logger';
	import { PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';

	type QuestionSet = {
		id: number | null;
		name: string;
		question_count: number;
	};

	let selectedSet = $state<number | null>(null);
	let isLoading = $state(false);
	let sets = $state<QuestionSet[]>([]);

	async function fetchQuestionSets() {
		isLoading = true;
		try {
			const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/list-sets`);
			if (!response.ok) throw new Error('Failed to fetch question sets');
			const data = await response.json();
			const totalQuestions = data.sets.reduce(
				(acc: number, set: any) => acc + set.question_count,
				0
			);
			sets = [{ id: null, name: 'All Questions', question_count: totalQuestions }, ...data.sets];
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
			await gameActions.createGame('Admin', selectedSet);
		} catch (err) {
			warn('Lobby creation failed:', err);
			notifications.add('Failed to create lobby', 'destructive');
		}
	}

	$effect(() => {
		fetchQuestionSets();
	});
</script>

<div class="flex h-[600px] flex-col">
	<ScrollArea class="flex-1">
		<div class="space-y-4 p-4">
			{#if isLoading}
				<div class="text-center text-muted-foreground">Loading question sets...</div>
			{:else}
				<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
					{#each sets as set (set.id)}
						<Card
							class={`cursor-pointer transition-colors hover:bg-accent/50 ${
								selectedSet === set.id ? 'border-2 !border-primary' : ''
							}`}
							onclick={() => (selectedSet = set.id)}
						>
							<CardContent class="p-4">
								<h3 class="font-semibold">{set.name}</h3>
								<p class="text-sm text-muted-foreground">
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

	<div class="border-t bg-card p-4">
		<div class="flex justify-end gap-2">
			<Button onclick={handleCreate} disabled={isLoading}>Create Lobby</Button>
		</div>
	</div>
</div>
