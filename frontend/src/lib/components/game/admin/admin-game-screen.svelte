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
	const error = $derived(gameStore.state.error);
</script>

<!-- Main container with height constraints -->
<div class="container mx-auto flex h-screen max-h-screen flex-col overflow-hidden p-3">
	<!-- Scrollable content area that will shrink if needed -->
	<div class="flex min-h-0 flex-1 flex-col space-y-4 lg:grid lg:grid-cols-12 lg:gap-4 lg:space-y-0">
		<!-- Left section -->
		<div class="lg:col-span-3 lg:flex lg:flex-col lg:space-y-4">
			<!-- Mobile: 2-column grid for top row, Desktop: stacked in left column -->
			<div class="grid grid-cols-2 gap-4 lg:grid-cols-1">
				<div class="space-y-2">
					<EndLeaveButton />
					<JoinCodeCard />
					<GameVideo />
				</div>
				<div class="lg:hidden">
					<UpcomingQuestions />
				</div>
			</div>
		</div>

		<!-- Middle section -->
		<div class="flex min-h-0 flex-col lg:col-span-6 lg:space-y-4">
			<!-- QuestionView - Always visible on desktop, only during question phase on mobile -->

			{#if phase === GamePhase.Question}
				<div class="flex min-h-0 flex-col space-y-4 lg:mt-0">
					<!-- QuestionView - Only visible on mobile during question phase -->
					<QuestionView />
					<div class="lg:grid lg:grid-cols-2 lg:gap-4">
						<AnswerProgress />
						<RoundTimer />
					</div>
				</div>
			{:else if showScoreboard}
				<div class="flex min-h-0 flex-1 lg:mt-0">
					<Scoreboard />
				</div>
			{:else if GamePhase.Lobby}
				<div class="mb-4 flex min-h-0 flex-1 lg:mt-0">
					<PlayersList />
				</div>
				<StartButton />
			{/if}
		</div>

		<!-- Right section - only visible on desktop -->
		<div class="hidden min-h-0 lg:col-span-3 lg:flex lg:flex-col lg:space-y-4">
			<div class="flex min-h-0 flex-1 flex-col space-y-4">
				<div class="h-64 rounded-lg bg-white p-4 shadow">
					<PlayersList />
				</div>
				<div class="flex-1 rounded-lg bg-white p-4 shadow">
					<UpcomingQuestions />
				</div>
			</div>
		</div>
	</div>

	<!-- Admin Controls - Always at bottom -->
	<div class="fixed bottom-0 left-0 right-0 z-10 mt-4 flex bg-white p-3">
		<SkipButton />
		<RoundButton />
	</div>
</div>
