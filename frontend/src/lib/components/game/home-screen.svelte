<script lang="ts">
	import { PUBLIC_SUPPORT_EMAIL, PUBLIC_TITLE } from '$app/env/public';
	import { resolve } from '$app/paths';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Card, CardContent, CardHeader, CardTitle } from '#lib/components/ui/card/index.ts';
	import { Separator } from '#lib/components/ui/separator/index.ts';
	import SetSelector from '#lib/components/set-selector.svelte';
	import LightSwitch from '#lib/components/ui/light-switch.svelte';
	import JoinLobbyCard from '#lib/components/join-lobby-card.svelte';
	import ReconnectCard from '#lib/components/reconnect-card.svelte';
	import { gameStore, type ValidatedSession } from '#lib/stores/game.svelte.ts';
	import {
		checkAndLoadSession,
		reconnectToSession as reconnectSession
	} from '#lib/utils/session.ts';
	import { onMount } from 'svelte';

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
				<a href={resolve('/howto')}><span class="text-muted-foreground">How to play</span></a>
				<Separator orientation="vertical" class="mx-2 h-4" />
				<Button
					href="https://github.com/kottz/spektrum"
					target="_blank"
					variant="outline"
					size="icon"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="24"
						height="24"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						class="h-[1.2rem] w-[1.2rem]"
						aria-hidden="true"
					>
						<path
							d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3.28-.36 6.72-1.61 6.72-7A5.4 5.4 0 0 0 19.22 4 5 5 0 0 0 19.13.5S17.95.14 15 1.84a13.4 13.4 0 0 0-7 0C5.05.14 3.87.5 3.87.5A5 5 0 0 0 3.78 4a5.4 5.4 0 0 0-1.5 3.5c0 5.42 3.44 6.66 6.72 7A4.8 4.8 0 0 0 8 18v4"
						></path>
						<path d="M9 18c-4.51 2-5-2-7-2"></path>
					</svg>
					<span class="sr-only">Github repository</span>
				</Button>
			</div>
			{#if PUBLIC_SUPPORT_EMAIL}
				<div class="text-muted-foreground mt-2 text-xs">
					Questions? <a class="ml-1" href={`mailto:${PUBLIC_SUPPORT_EMAIL}`}
						>{PUBLIC_SUPPORT_EMAIL}</a
					>
				</div>
			{/if}
		</footer>
	</div>
</div>
