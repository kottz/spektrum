<script lang="ts">
	let { children } = $props();
</script>

<svelte:head>
	<title>Melodiquiz - Stream View</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="h-dvh">
	{@render children()}
</div>

<style>
	:global(body.stream-view-active) {
		overflow: hidden;
	}
</style>
