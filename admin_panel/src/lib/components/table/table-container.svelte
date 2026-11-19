<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		showHeader?: boolean;
		headerLeft?: Snippet;
		headerRight?: Snippet;
		children: Snippet;
	}

	let { showHeader = true, headerLeft, headerRight, children }: Props = $props();
</script>

<div class="h-full rounded-md border">
	{#if showHeader}
		<div class="m-4 flex items-center justify-between">
			{@render headerLeft?.()}
			{@render headerRight?.()}
		</div>
	{/if}

	<div class="h-[calc(100%-5rem)] rounded-md border">
		{@render children()}
	</div>
</div>
