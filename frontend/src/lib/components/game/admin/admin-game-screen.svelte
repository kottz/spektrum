<script lang="ts">
    import { gameStore } from '$lib/stores/game.svelte';
    import { GamePhase } from '$lib/types/game';
    import JoinCodeCard from '$lib/components/game/admin/join-code-card.svelte';
    import GameVideo from '$lib/components/game/admin/youtube-player.svelte';
    import GameControls from '$lib/components/game/admin/game-controls.svelte';
    import PlayersList from '$lib/components/game/admin/player-list.svelte';
    import Scoreboard from '$lib/components/game/scoreboard.svelte';
    import QuestionView from '$lib/components/game/admin/admin-question-view.svelte';
	import UpcomingQuestions from '$lib/components/game/admin/upcoming-questions.svelte';

    const phase = $derived(gameStore.state.phase);
    const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
    const error = $derived(gameStore.state.error);
</script>

<div class="container mx-auto max-w-md p-4 pb-48">
    <div class="space-y-4">
        <JoinCodeCard />
        <GameVideo />
        
        {#if phase === GamePhase.Question}
            <QuestionView />
        {/if}

        <PlayersList />
        
        {#if showScoreboard}
            <Scoreboard />
        {/if}

        {#if error}
            <div class="p-4 bg-red-100 text-red-700 rounded-md">
                {error}
            </div>
        {/if}
    </div>
    <UpcomingQuestions />
    <GameControls />
</div>
