<script>
	import { gameStore } from '$lib/stores/game.svelte';
	import { GamePhase } from '$lib/types/game';
	import JoinCodeCard from '$lib/components/game/admin/join-code-card.svelte';
	import GameVideo from '$lib/components/game/admin/youtube-player.svelte';
	import PlayersList from '$lib/components/game/admin/player-list.svelte';
	import Scoreboard from '$lib/components/game/scoreboard.svelte';
	import QuestionView from '$lib/components/game/admin/admin-question-view.svelte';
	import UpcomingQuestions from '$lib/components/game/admin/upcoming-questions.svelte';
	import RoundTimer from '$lib/components/game/round-timer.svelte';
	import AnswerProgress from '$lib/components/game/answer-progress.svelte';
	import StartButton from '$lib/components/game/admin/controls/start-game-button.svelte';
	import EndLeaveButton from '$lib/components/game/admin/controls/end-leave-button.svelte';
	import RoundButton from '$lib/components/game/admin/controls/round-control-button.svelte';
	import SkipButton from '$lib/components/game/admin/controls/skip-control-button.svelte';
	import CoverableElement from '$lib/components/coverable-element.svelte';

	const phase = $derived(gameStore.state.phase);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
	let hideGameContent = $state(false);
</script>

<!-- Main container with padding -->
<div class="container mx-auto flex min-h-screen flex-col p-3 pb-24 lg:pb-3">
	<div class="flex min-h-0 flex-1 flex-col space-y-4 lg:grid lg:grid-cols-12 lg:gap-4 lg:space-y-0">
		<!-- Left column -->
		<div class="lg:col-span-4">
			<div class="grid grid-cols-2 gap-4 lg:block lg:space-y-4">
				<!-- Grouping controls and join code -->
				<div class="space-y-4">
					<EndLeaveButton />
					<JoinCodeCard />
					<!-- Coverable YouTube Player -->
					<div class="aspect-video w-full">
						<CoverableElement covered={hideGameContent} coverText="Video Hidden">
							{#snippet children()}
								<GameVideo />
							{/snippet}
						</CoverableElement>
					</div>
				</div>
				<!-- Grouping upcoming questions and spoiler toggle (on desktop) -->
				<div class="space-y-4">
					<!-- Coverable Upcoming Questions -->
					<CoverableElement covered={hideGameContent} coverText="Questions Hidden">
						{#snippet children()}
							<UpcomingQuestions />
						{/snippet}
					</CoverableElement>

					<!-- Desktop-only spoiler mode toggle and controls -->
					<div class="hidden lg:block">
						<div class="mb-2 flex select-none items-center gap-2 rounded">
							<input
								type="checkbox"
								id="spoiler-mode"
								bind:checked={hideGameContent}
								class="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary"
							/>
							<!-- Use bind:checked for two-way binding -->
							<label for="spoiler-mode" class="cursor-pointer text-sm font-medium"
								>No Spoiler Mode</label
							>
						</div>

						{#if phase === GamePhase.Lobby}
							<StartButton />
						{:else}
							<div class="flex gap-2">
								<SkipButton />
								<RoundButton />
							</div>
						{/if}
						<div class="mt-4 space-y-4">
							<AnswerProgress />
							<RoundTimer />
						</div>
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
						<!-- Moved timer/progress here for mobile Question phase -->
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
					<!-- Combined Question & Scoreboard for non-lobby desktop -->
					<div class="mb-4 flex-none">
						<!-- Coverable Question View (Desktop) -->
						<CoverableElement covered={hideGameContent} coverText="Question Hidden">
							{#snippet children()}
								<QuestionView />
							{/snippet}
						</CoverableElement>
					</div>
					<div class="flex min-h-0 flex-1">
						<Scoreboard />
					</div>
				{/if}
			</div>
		</div>
	</div>

	<!-- Mobile-only bottom controls -->
	<div class="fixed bottom-0 left-0 right-0 z-10 bg-background/95 p-3 backdrop-blur-sm lg:hidden">
		<!-- Spoiler Toggle for Mobile -->
		<div class="mb-3 flex select-none items-center justify-end gap-2 rounded text-xs">
			<label for="spoiler-mode-mobile" class="cursor-pointer font-medium">No Spoilers</label>
			<input
				type="checkbox"
				id="spoiler-mode-mobile"
				bind:checked={hideGameContent}
				class="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary"
			/>
		</div>
		<!-- Phase Controls -->
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
