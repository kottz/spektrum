<script lang="ts">
	import type { PublicGameState } from '$lib/types/game';
	import { GamePhase } from '$lib/types/game';
	import StreamQuestionView from './stream-question-view.svelte';
	import StreamScoreboard from './stream-scoreboard.svelte';
	import StreamAnswerProgress from './stream-answer-progress.svelte';

	interface Props {
		gameState: PublicGameState | null;
	}
	let { gameState }: Props = $props();

	const phase = $derived(gameState?.phase.type);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
</script>

{#if gameState}
	<div class="container mx-auto flex h-screen flex-col p-3">
		<!-- Header section: Join Code, Phase Info -->
		<header class="mb-4 flex-none">
			<div class="flex items-center justify-between rounded-lg bg-card p-4 shadow">
				<div>
					<span class="text-muted-foreground">Join Code: </span>
					<span class="font-mono text-xl font-bold">{gameState.joinCode || 'N/A'}</span>
				</div>
				<div class="text-lg font-semibold capitalize">
					{phase}
				</div>
			</div>
		</header>

		<!-- Main Content Area -->
		<div class="grid min-h-0 flex-1 grid-cols-12 gap-4">
			<!-- Left Column: Answer Progress, Timer (if any) -->
			<div class="col-span-12 space-y-4 lg:col-span-4">
				{#if phase === GamePhase.Question}
					<StreamAnswerProgress {gameState} />
					<!-- Timer component could be added here if needed -->
				{/if}
				{#if phase === GamePhase.Lobby}
					<div class="flex h-full items-center justify-center rounded-lg bg-card p-4 shadow">
						<p class="text-xl text-muted-foreground">Waiting for game to start...</p>
					</div>
				{/if}
			</div>

			<!-- Right Column: Question or Scoreboard -->
			<div class="col-span-12 lg:col-span-8">
				{#if phase === GamePhase.Question}
					<StreamQuestionView {gameState} />
				{:else if showScoreboard}
					<StreamScoreboard {gameState} />
				{:else if phase === GamePhase.Lobby}
					<!-- Show lobby players via scoreboard -->
					<StreamScoreboard {gameState} />
				{/if}
			</div>
		</div>
	</div>
{:else}
	<div class="flex h-full items-center justify-center">
		<p>Loading Game View...</p>
	</div>
{/if}
