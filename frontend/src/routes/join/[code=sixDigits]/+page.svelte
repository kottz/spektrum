<script lang="ts">
	import { PUBLIC_TITLE } from '$env/static/public';
	import { Button } from '$lib/components/ui/button';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	import JoinLobbyByCode from '$lib/components/join-lobby-by-code.svelte';
	import ReconnectCard from '$lib/components/reconnect-card.svelte';
	import { House } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { gameStore, type ValidatedSession } from '$lib/stores/game.svelte';
	import { checkAndLoadSession, reconnectToSession as reconnectSession } from '$lib/utils/session';
	import { onMount } from 'svelte';

	let currentSession = $state<ValidatedSession | null>(null);

	onMount(async () => {
		currentSession = await checkAndLoadSession();
	});

	function reconnectToSession() {
		if (!currentSession) return;
		reconnectSession(currentSession);
		goto('/');
	}

	function handleNewLobby() {
		gameStore.cleanup();
		currentSession = null;
	}
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-8 px-3 py-8">
	<div class="relative w-full max-w-lg">
		<!-- House button absolutely positioned on the left -->
		<div class="absolute top-1/2 left-0 -translate-y-1/2">
			<Button class="ml-2" variant="outline" size="icon" onclick={() => goto('/')}>
				<House class="h-[1.2rem] w-[1.2rem]" />
				<span class="sr-only">Go home</span>
			</Button>
		</div>
		<!-- Centered title group -->
		<div class="flex items-center justify-center gap-3">
			<span class="text-2xl">🎵</span>
			<h1 class="text-3xl font-bold">{PUBLIC_TITLE}</h1>
			<LightSwitch />
		</div>
	</div>
	<div class="grid w-full max-w-lg gap-6">
		{#if currentSession && !currentSession.isAdmin}
			<ReconnectCard
				session={currentSession}
				onReconnect={reconnectToSession}
				onNewLobby={handleNewLobby}
			/>
		{:else}
			<JoinLobbyByCode initialJoinCode={$page.params.code ?? ''} />
		{/if}
	</div>
</div>
