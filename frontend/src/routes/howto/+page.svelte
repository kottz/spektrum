<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import LightSwitch from '$lib/components/ui/light-switch.svelte';
	import NotificationList from '$lib/components/NotificationList.svelte';
	import { Separator } from '$lib/components/ui/separator';
	import { House } from 'lucide-svelte';
	import { goto } from '$app/navigation';
</script>

<div class="container flex min-h-screen flex-col items-center justify-center gap-8 py-8">
	<NotificationList />
	<div class="grid w-full max-w-lg gap-6">
		<div class="flex items-center justify-center gap-3">
			<Button class="ml-2" variant="outline" size="icon" onclick={() => goto('/')}>
				<House class="h-[1.2rem] w-[1.2rem]" />
				<span class="sr-only">Go home</span>
			</Button>
			<LightSwitch />
		</div>
		<h1 class="text-3xl font-bold">How to Play</h1>
		<Separator />
		<p>
			The game is meant to be played with all participants in the same room. One person acts as the
			host and is responsible for music playback. The host creates a lobby and shares the join code
			with the group. When players have joined, the host starts the game.
		</p>
		<p>
			Players will hear a piece of music and must identify the correct color or character associated
			with each song. Quick answers score more points.
		</p>
		<p>Some questions have multiple correct answers.</p>
		<h1 class="text-3xl font-bold">FAQ</h1>
		<Separator />
		<h2 class="text-l font-bold">
			How do I get rid of <i
				>"Playback paused because your account is being used in another location"</i
			> errors?
		</h2>
		<p>
			Manually click the play button in the embedded YouTube player before starting the next round.
			You only have to click the player manually once, after that you can use the start round button
			to play the music as normal.
		</p>
		<p>Don't use your YouTube account in other places while the game is active.</p>
		<h2 class="text-l font-bold">Why do the colors not look correct?</h2>
		<p>
			If you use Dark Reader or similar browser extensions, they may interfere with the colors.
			Disable them for this page and use the built-in dark mode toggle.
		</p>
	</div>
</div>
