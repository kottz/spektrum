<script lang="ts">
	import { ModeWatcher } from 'mode-watcher';
	import NotificationList from '#lib/components/NotificationList.svelte';
	import '../app.css';
	import { PUBLIC_TITLE, PUBLIC_UMAMI_WEBSITE_ID } from '$app/env/public';
	let { children } = $props();
</script>

<svelte:head>
	<title>{PUBLIC_TITLE}</title>
	{#if PUBLIC_UMAMI_WEBSITE_ID}
		<script
			defer
			src="https://cloud.umami.is/script.js"
			data-website-id={PUBLIC_UMAMI_WEBSITE_ID}
		></script>
	{/if}
</svelte:head>

<ModeWatcher defaultMode="dark" />
<NotificationList />
{@render children()}
