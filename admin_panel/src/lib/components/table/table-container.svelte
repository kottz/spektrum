<script lang="ts">
	let { showHeader = true } = $props<{
		showHeader?: boolean;
	}>();
</script>

<div class="rounded-md border">
	{#if showHeader}
		<div class="mb-4 flex items-center justify-between">
			<slot name="header-left" />
			<slot name="header-right" />
		</div>
	{/if}

	<div class="h-[calc(100%-5rem)] rounded-md border">
		<slot />
	</div>
</div>
