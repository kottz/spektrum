<script lang="ts">
	import { gameStore } from '../../stores/game';
	import AdminControls from './admin-controls.svelte';
	import Scoreboard from './scoreboard.svelte';
	import { Card } from '$lib/components/ui/card';
	import { GamePhase } from '../../types/game';
	import YouTubePlayer from './youtube-player.svelte';
	import AdminQuestionView from './admin-question-view.svelte'; // Changed import

	const joinCode = $derived($gameStore.joinCode);
	const phase = $derived($gameStore.phase);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
</script>

<div class="container mx-auto p-6">
	<div class="grid grid-cols-[1fr,300px] gap-6">
		<!-- Main content -->
		<div class="space-y-6">
			<!-- Join code -->
			{#if joinCode}
				<Card>
					<div class="flex items-center justify-between p-4">
						<div class="text-muted-foreground">Join Code:</div>
						<div class="font-mono text-lg font-bold">{joinCode}</div>
					</div>
				</Card>
			{/if}
			<!-- Scoreboard (shown in score phase and game over) -->
			{#if showScoreboard}
				<Card>
					<div class="p-6">
						<Scoreboard />
					</div>
				</Card>
			{/if}
			<!-- YouTube player -->
			<YouTubePlayer />
			<!-- Question preview - using new AdminQuestionView -->
			{#if phase === GamePhase.Question}
				<AdminQuestionView />
			{/if}
		</div>
		<!-- Admin controls sidebar -->
		<div class="space-y-6">
			<AdminControls />
			{#if $gameStore.error}
				<Card class="border-red-500/20 bg-red-500/10">
					<div class="p-4 text-red-500">
						{$gameStore.error}
					</div>
				</Card>
			{/if}
		</div>
	</div>
</div>
