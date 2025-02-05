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

<div class="container mx-auto flex h-screen max-h-screen flex-col overflow-hidden p-3">
	<div class="flex min-h-0 flex-1 flex-col space-y-4 lg:grid lg:grid-cols-12 lg:gap-4 lg:space-y-0">
		<!-- Left column that contains shared elements -->
		<div class="lg:col-span-4">
			<!-- These elements are always in the left column -->
			<div class="space-y-4 lg:block {phase === GamePhase.Lobby ? 'block' : 'grid grid-cols-2 gap-4'}">
				<div class="space-y-4">
					<EndLeaveButton />
					<JoinCodeCard />
					<div class="aspect-video w-full">
						<GameVideo />
					</div>
				</div>
				
				<!-- UpcomingQuestions appears right on mobile, bottom on desktop -->
				<div class="lg:mt-4">
					<UpcomingQuestions />
				</div>

				<!-- Desktop-only controls -->
				<div class="hidden lg:block space-y-4">
					<div class="flex gap-2">
						<SkipButton />
						<RoundButton />
						<StartButton />
					</div>
					<div class="grid grid-cols-2 gap-4">
						<AnswerProgress />
						<RoundTimer />
					</div>
				</div>
			</div>
		</div>

		<!-- Right column content -->
		<div class="lg:col-span-8">
			{#if phase === GamePhase.Question}
				<div class="flex min-h-0 flex-col space-y-4">
					<QuestionView />
					<!-- Mobile-only timer and progress -->
					<div class="grid grid-cols-2 gap-4 lg:hidden">
						<AnswerProgress />
						<RoundTimer />
					</div>
				</div>
			{:else if showScoreboard}
				<div class="flex min-h-0 flex-1">
					<Scoreboard />
				</div>
			{:else if phase === GamePhase.Lobby}
				<div class="rounded-lg bg-white p-4 shadow">
					<PlayersList />
				</div>
			{/if}
		</div>
	</div>

	<!-- Mobile-only bottom controls -->
	<div class="fixed bottom-0 left-0 right-0 z-10 mt-4 flex bg-white p-3 lg:hidden">
		<SkipButton />
		<RoundButton />
		<StartButton />
	</div>
</div>
