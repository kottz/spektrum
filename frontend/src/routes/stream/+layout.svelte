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

<div class="stream-window-container bg-background text-foreground h-dvh">
	{@render children()}
</div>

<style>
	:global(body.stream-view-active) {
		overflow: hidden;
	}
	.stream-window-container {
		min-height: 100vh;
	}
</style>
