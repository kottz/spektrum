<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	import JoinLobbyByCode from '$lib/components/join-lobby-by-code.svelte';
	import NotificationList from '$lib/components/NotificationList.svelte';
	import { House } from 'lucide-svelte';
	import { goto } from '$app/navigation';

	const { data } = $props<{ data: { joinCode: string } }>();
</script>

<div class="container flex min-h-screen flex-col items-center justify-center gap-8 py-8">
	<NotificationList />
	<div class="relative w-full max-w-lg">
		<!-- House button absolutely positioned on the left -->
		<div class="absolute left-0 top-1/2 -translate-y-1/2">
			<Button class="ml-2" variant="outline" size="icon" onclick={() => goto('/')}>
				<House class="h-[1.2rem] w-[1.2rem]" />
				<span class="sr-only">Go home</span>
			</Button>
		</div>
		<!-- Centered title group -->
		<div class="flex items-center justify-center gap-3">
			<span class="text-2xl">🎵</span>
			<h1 class="text-3xl font-bold">Music Quiz</h1>
			<LightSwitch />
		</div>
	</div>
	<div class="grid w-full max-w-lg gap-6">
		<JoinLobbyByCode initialJoinCode={data.joinCode} />
	</div>
</div>
