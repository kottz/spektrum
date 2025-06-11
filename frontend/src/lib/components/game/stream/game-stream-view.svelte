<script lang="ts">
	import { GamePhase } from '$lib/types/game';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import StreamQuestionView from './stream-question-view.svelte';
	import StreamScoreboard from './stream-scoreboard.svelte';
	import StreamAnswerProgress from './stream-answer-progress.svelte';

	const gameState = $derived(streamStore.state.gameState);
	const phase = $derived(gameState?.phase);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
	const joinCode = $derived(gameState?.joinCode || 'N/A');
</script>

{#if gameState}
	<div class="container mx-auto flex h-screen flex-col p-3">
		<!-- Header section: Join Code, Phase Info -->
		<header class="mb-4 flex-none">
			<div class="flex items-center justify-between rounded-lg bg-card p-4 shadow">
				<div>
					<span class="text-muted-foreground">Join Code: </span>
					<span class="font-mono text-xl font-bold">{joinCode}</span>
				</div>
				<div class="text-lg font-semibold capitalize">
					{phase}
				</div>
			</div>
		</header>

		<!-- Main Content Area -->
		<div class="grid min-h-0 flex-1 grid-cols-12 gap-4">
			<!-- Left Column: Question Options + Answer Progress -->
			<div class="col-span-12 flex flex-col space-y-4 lg:col-span-6">
				<!-- Question Options (top) -->
				<div class="flex-none">
					{#if phase === GamePhase.Question}
						<StreamQuestionView />
					{:else if phase === GamePhase.Lobby}
						<div class="flex h-48 items-center justify-center rounded-lg bg-card p-4 shadow">
							<p class="text-xl text-muted-foreground">Waiting for game to start...</p>
						</div>
					{:else}
						<div class="flex h-48 items-center justify-center rounded-lg bg-card p-4 shadow">
							<p class="text-xl text-muted-foreground">Round Over</p>
						</div>
					{/if}
				</div>

				<!-- Answer Progress (bottom) -->
				<div class="min-h-0 flex-1">
					<StreamAnswerProgress />
				</div>
			</div>

			<!-- Right Column: Always-visible Scoreboard -->
			<div class="col-span-12 lg:col-span-6">
				<StreamScoreboard />
			</div>
		</div>
	</div>
{:else}
	<div class="flex h-full items-center justify-center">
		<p>Loading Game View...</p>
	</div>
{/if}
