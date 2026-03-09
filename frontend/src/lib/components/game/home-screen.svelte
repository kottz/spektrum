<script lang="ts">
	import { PUBLIC_TITLE } from '$env/static/public';
	import { env } from '$env/dynamic/public';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Separator } from '$lib/components/ui/separator';
	import { Github } from '@lucide/svelte';
	import SetSelector from '$lib/components/set-selector.svelte';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	import JoinLobbyCard from '$lib/components/join-lobby-card.svelte';
	import ReconnectCard from '$lib/components/reconnect-card.svelte';
	import { gameStore, type ValidatedSession } from '$lib/stores/game.svelte';
	import { checkAndLoadSession, reconnectToSession as reconnectSession } from '$lib/utils/session';
	import { onMount } from 'svelte';

	const supportEmail = env.PUBLIC_SUPPORT_EMAIL?.trim();
	let showSetSelector = $state(false);
	let showJoinCard = $state(false);
	let currentSession = $state<ValidatedSession | null>(null);

	onMount(async () => {
		currentSession = await checkAndLoadSession();
	});

	function reconnectToSession() {
		if (!currentSession) return;
		reconnectSession(currentSession);
	}

	function handleNewLobby() {
		gameStore.cleanup();
		showJoinCard = true;
		currentSession = null;
	}
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-8 px-3 py-8">
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
						<Button variant="outline" size="sm" onclick={() => (showSetSelector = false)}>
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
					<Button size="lg" class="w-full" onclick={() => (showSetSelector = true)}>
						Create Lobby
					</Button>
				{/if}
			</CardContent>
		</Card>

		{#if !showSetSelector}
			{#if currentSession && !showJoinCard}
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
		<footer class="flex flex-col items-center justify-center text-sm">
			<div class="flex items-center justify-center">
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
			</div>
			{#if supportEmail}
				<div class="text-muted-foreground mt-2 text-xs">
					Questions? <a class="ml-1" href={`mailto:${supportEmail}`}>{supportEmail}</a>
				</div>
			{/if}
		</footer>
	</div>
</div>
