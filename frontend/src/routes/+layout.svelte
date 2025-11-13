<script lang="ts">
	import { ModeWatcher } from 'mode-watcher';
	import '../app.css';
	import { PUBLIC_TITLE } from '$env/static/public';
	import { env } from '$env/dynamic/public';
	let { children } = $props();
</script>

<svelte:head>
	<title>{PUBLIC_TITLE}</title>
	{#if env.PUBLIC_UMAMI_WEBSITE_ID}
		<script
			defer
			src="https://cloud.umami.is/script.js"
			data-website-id={env.PUBLIC_UMAMI_WEBSITE_ID}
		></script>
	{/if}
</svelte:head>

<ModeWatcher defaultMode={'dark'} />
{@render children()}
