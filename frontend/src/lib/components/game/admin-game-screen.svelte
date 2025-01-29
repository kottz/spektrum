<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import AdminControls from './admin-controls.svelte';
	import Scoreboard from './scoreboard.svelte';
	import { Card } from '$lib/components/ui/card';
	import { GamePhase } from '$lib/types/game';
	import YouTubePlayer from './youtube-player.svelte';
	import AdminQuestionView from './admin-question-view.svelte';

	const joinCode = $derived(gameStore.state.joinCode);
	const error = $derived(gameStore.state.error);
	const phase = $derived(gameStore.state.phase);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
</script>

<div class="container mx-auto p-6">
	<div class="grid grid-cols-[1fr,300px] gap-6">
		<div class="space-y-6">
			{#if joinCode}
				<Card>
					<div class="flex items-center justify-between p-4">
						<div class="text-muted-foreground">Join Code:</div>
						<div class="font-mono text-lg font-bold">{joinCode}</div>
					</div>
				</Card>
			{/if}

			<YouTubePlayer />

			{#if showScoreboard}
				<Card>
					<div class="p-6">
						<Scoreboard />
					</div>
				</Card>
			{/if}
			<!-- Question preview - using new AdminQuestionView -->
			{#if phase === GamePhase.Question}
				<AdminQuestionView />
			{/if}
		</div>

		<div class="space-y-6">
			<AdminControls />
			{#if error}
				<Card class="border-red-500/20 bg-red-500/10">
					<div class="p-4 text-red-500">
						{error}
					</div>
				</Card>
			{/if}
		</div>
	</div>
</div>
