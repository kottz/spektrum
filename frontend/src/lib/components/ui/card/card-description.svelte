<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type CardDescriptionProps = WithElementRef<HTMLAttributes<HTMLParagraphElement>> & {
		children?: Snippet;
	};

	let {
		class: className,
		ref = $bindable(null),
		children,
		...restProps
	}: CardDescriptionProps = $props();
</script>

<p bind:this={ref} class={cn('text-muted-foreground text-sm', className)} {...restProps}>
	{@render children?.()}
</p>
