<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { GamePhase } from '$lib/types/game';
	import JoinCodeCard from '$lib/components/game/admin/join-code-card.svelte';
	import GameVideo from '$lib/components/game/admin/youtube-player.svelte';
	import PlayersList from '$lib/components/game/admin/player-list.svelte';
	import Scoreboard from '$lib/components/game/scoreboard.svelte';
	import QuestionView from '$lib/components/game/admin/admin-question-view.svelte';
	import UpcomingQuestions from '$lib/components/game/admin/upcoming-questions.svelte';
	import RoundTimer from '../round-timer.svelte';
	import AnswerProgress from '../answer-progress.svelte';
	import StartButton from './controls/start-game-button.svelte';
	import EndLeaveButton from './controls/end-leave-button.svelte';
	import RoundButton from './controls/round-control-button.svelte';
	import SkipButton from './controls/skip-control-button.svelte';

	const phase = $derived(gameStore.state.phase);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
</script>

<div class="container mx-auto flex h-screen flex-col overflow-hidden p-3">
	<div class="flex min-h-0 flex-1 flex-col space-y-4 lg:grid lg:grid-cols-12 lg:gap-4 lg:space-y-0">
		<!-- Left column -->
		<div class="lg:col-span-4">
			<div class="grid grid-cols-2 gap-4 lg:block lg:space-y-4">
				<div class="space-y-4">
					<EndLeaveButton />
					<JoinCodeCard />
					<div class="aspect-video w-full">
						<GameVideo />
					</div>
				</div>

				<div class="lg:mt-4">
					<UpcomingQuestions />
				</div>

				<div class="hidden space-y-4 lg:block">
					{#if phase === GamePhase.Lobby}
						<StartButton />
					{:else}
						<div class="flex gap-2">
							<SkipButton />
							<RoundButton />
						</div>
					{/if}
					<div class="space-y-4">
						<AnswerProgress />
						<RoundTimer />
					</div>
				</div>
			</div>
		</div>

		<!-- Right column -->
		<div class="flex min-h-0 flex-1 flex-col lg:col-span-8">
			<!-- Mobile Layout -->
			<div class="flex min-h-0 flex-1 flex-col lg:hidden">
				{#if phase === GamePhase.Question}
					<div class="flex min-h-0 flex-1 flex-col space-y-4">
						<QuestionView />
						<div class="space-y-4">
							<AnswerProgress />
							<RoundTimer />
						</div>
					</div>
				{:else if showScoreboard}
					<div class="flex min-h-0 flex-1">
						<Scoreboard />
					</div>
				{:else if phase === GamePhase.Lobby}
					<div class="flex min-h-0 flex-1">
						<PlayersList />
					</div>
				{/if}
			</div>

			<!-- Desktop Layout -->
			<div class="hidden min-h-0 flex-1 flex-col lg:flex">
				{#if phase === GamePhase.Lobby}
					<div class="flex min-h-0 flex-1">
						<PlayersList />
					</div>
				{:else}
					<div class="mb-4 flex-none">
						<QuestionView />
					</div>
					<div class="flex min-h-0 flex-1">
						<Scoreboard />
					</div>
				{/if}
			</div>
		</div>
	</div>

	<!-- Mobile-only bottom controls -->
	<div class="fixed bottom-0 left-0 right-0 z-10 mt-4 flex bg-background p-3 lg:hidden">
		{#if phase === GamePhase.Lobby}
			<StartButton />
		{:else}
			<div class="flex w-full gap-2">
				<SkipButton />
				<RoundButton />
			</div>
		{/if}
	</div>
</div>
