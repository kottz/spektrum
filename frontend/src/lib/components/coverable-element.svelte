<script>
	let { covered = false, coverColor = 'bg-background/100', coverText = '', children } = $props();
</script>

<div class="relative h-full w-full overflow-hidden rounded">
	<div
		class="transition-filter relative h-full w-full duration-300 ease-in-out {covered
			? 'blur-sm'
			: ''}"
	>
		{@render children()}
	</div>

	{#if covered}
		<div
			class="pointer-events-none absolute inset-0 z-20 flex items-center justify-center rounded {coverColor} motion-safe:animate-fade-in transition-opacity duration-300 ease-in-out"
		>
			{#if coverText}
				<span class="font-bold text-foreground/80">{coverText}</span>
			{/if}
		</div>
	{/if}
</div>

<style>
	@keyframes fade-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}
	.animate-fade-in {
		/* Ensures the fade-in happens when the #if block adds the element */
		animation: fade-in 0.3s ease-in-out forwards;
	}
</style>
