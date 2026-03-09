<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type CardProps = WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		children?: Snippet;
	};

	let { class: className, ref = $bindable(null), children, ...restProps }: CardProps = $props();
</script>

<div
	bind:this={ref}
	class={cn('border-border bg-card text-card-foreground rounded-lg border shadow-sm', className)}
	{...restProps}
>
	{@render children?.()}
</div>
