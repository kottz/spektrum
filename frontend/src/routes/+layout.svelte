<script lang="ts">
	import { ModeWatcher } from 'mode-watcher';
	import '../app.css';
	import { PUBLIC_UMAMI_WEBSITE_ID } from '$env/static/public';
	let { children } = $props();
</script>

<svelte:head>
	{#if PUBLIC_UMAMI_WEBSITE_ID}
		<script
			defer
			src="https://cloud.umami.is/script.js"
			data-website-id={PUBLIC_UMAMI_WEBSITE_ID}
		></script>
	{/if}
</svelte:head>

<ModeWatcher defaultMode={'dark'} />
{@render children()}
