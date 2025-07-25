<script lang="ts">
	import { PUBLIC_TITLE } from '$env/static/public';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Separator } from '$lib/components/ui/separator';
	import { Github } from 'lucide-svelte';
	import SetSelector from '$lib/components/set-selector.svelte';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	import JoinLobbyCard from '$lib/components/join-lobby-card.svelte';
	import ReconnectCard from '$lib/components/reconnect-card.svelte';
	import { gameActions } from '$lib/stores/game-actions';
	import { gameStore } from '$lib/stores/game.svelte';
	import type { SessionInfo } from '$lib/stores/game.svelte';
	import { info, warn } from '$lib/utils/logger';

	let showSetSelector = $state(false);
	let showJoinCard = $state(false);
	let currentSession = $state<(SessionInfo & { last_update: string }) | null>(null);

	$effect(() => {
		(async () => {
			try {
				const session = await gameStore.checkSessions();
				info('Checking sessions on home load:', session);
				// Now TypeScript knows both types match exactly
				currentSession = session;
			} catch (error) {
				warn('Failed to check sessions:', error);
				currentSession = null;
			}
		})();
	});

	function reconnectToSession() {
		if (!currentSession) return;

		gameStore.setPlayerId(currentSession.playerId);
		gameStore.setAdminTo(currentSession.isAdmin);
		gameStore.setPlayerName(currentSession.playerName);
		gameStore.setJoinCode(currentSession.joinCode);
		gameActions.joinGame(currentSession.playerId);
	}

	function handleNewLobby() {
		gameStore.cleanup();
		showJoinCard = true;
		currentSession = null;
	}
</script>

<div class="container flex min-h-screen flex-col items-center justify-center gap-8 py-8">
	<div class="flex items-center gap-3">
		<span class="text-2xl">🎵</span>
		<h1 class="text-3xl font-bold">{PUBLIC_TITLE}</h1>
		<LightSwitch />
	</div>
	<div class="grid w-full max-w-lg gap-6">
		<Card>
			<CardHeader class="mb-2">
				<div class="flex items-center gap-4">
					{#if showSetSelector}
						<Button variant="outline" size="sm" on:click={() => (showSetSelector = false)}>
							Back
						</Button>
					{/if}
					<CardTitle>{showSetSelector ? 'Select Question Set' : 'Create New Lobby'}</CardTitle>
				</div>
			</CardHeader>
			<CardContent class={showSetSelector ? 'h-[400px] p-0' : 'p-4'}>
				{#if showSetSelector}
					<SetSelector />
				{:else}
					<Button size="lg" class="w-full" on:click={() => (showSetSelector = true)}>
						Create Lobby
					</Button>
				{/if}
			</CardContent>
		</Card>

		{#if !showSetSelector}
			{#if currentSession && !showJoinCard}
				<h1>Current Session: {currentSession.playerName}</h1>
				<ReconnectCard
					session={currentSession}
					onReconnect={reconnectToSession}
					onNewLobby={handleNewLobby}
				/>
			{:else}
				<JoinLobbyCard />
			{/if}
		{/if}
		<Separator class="mt-4" />
		<footer class="flex items-center justify-center text-sm">
			<a href="/howto"><span class="text-muted-foreground">How to play</span></a>
			<Separator orientation="vertical" class="mx-2 h-4" />
			<Button
				href="https://github.com/kottz/spektrum"
				target="_blank"
				variant="outline"
				size="icon"
			>
				<Github class="h-[1.2rem] w-[1.2rem]" />
				<span class="sr-only">Github repository</span>
			</Button>
		</footer>
	</div>
</div>
