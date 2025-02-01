<script lang="ts">
	import { gameStore } from '$lib/stores/game.svelte';
	import { GamePhase } from '$lib/types/game';
	import JoinCodeCard from '$lib/components/game/admin/join-code-card.svelte';
	import GameVideo from '$lib/components/game/admin/youtube-player.svelte';
	import AdminControls from '$lib/components/game/admin/admin-controls.svelte';
	import PlayersList from '$lib/components/game/admin/player-list.svelte';
	import Scoreboard from '$lib/components/game/scoreboard.svelte';
	import QuestionView from '$lib/components/game/admin/admin-question-view.svelte';
	import UpcomingQuestions from '$lib/components/game/admin/upcoming-questions.svelte';
	import RoundTimer from '../round-timer.svelte';
	import AnswerProgress from '../answer-progress.svelte';

	const phase = $derived(gameStore.state.phase);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
	const error = $derived(gameStore.state.error);
</script>

<div class="container mx-auto space-y-4 p-4">
	<!-- Top Row: Two columns for Join Code and YouTube Player -->
	<div class="grid grid-cols-2 gap-4 md:grid-cols-2">
		<div class="grid grid-cols-1 items-center gap-2">
			<JoinCodeCard />
			<GameVideo />
		</div>
		<UpcomingQuestions />
	</div>

	{#if phase === GamePhase.Question}
		<!-- In Question Mode -->
		<!-- Row: Question Alternatives -->
		<div>
			<QuestionView />
		</div>

		<!-- Row: Answer Progress -->
		<div>
			<AnswerProgress />
		</div>

		<!-- Row: Timer -->
		<div>
			<RoundTimer />
		</div>
	{:else if phase === GamePhase.Score}
		<!-- In Score Mode -->
		<!-- Row: Two columns for Players List and Upcoming Questions -->
		<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
			<Scoreboard />
		</div>
	{/if}

	<!-- Last Row: Admin Controls -->
	<div>
		<AdminControls />
	</div>
</div>
