<script lang="ts">
	import '../../app.css';
	import { ModeWatcher } from 'mode-watcher';

	let { children } = $props();
</script>

<svelte:head>
	<title>Melodiquiz - Stream View</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<ModeWatcher defaultMode={'dark'} />

<div class="stream-window-container h-[100dvh] bg-background text-foreground">
	{@render children()}
</div>

<style>
	:global(body.stream-view-active) {
		overflow: hidden;
	}
	.stream-window-container {
		/* Additional stream-specific styles if needed */
	}
</style>
