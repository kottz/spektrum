<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { type Variant, alertVariants } from './index.js';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type AlertProps = WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		variant?: Variant;
		children?: Snippet;
	};

	let {
		class: className,
		variant = 'default',
		ref = $bindable(null),
		children,
		...restProps
	}: AlertProps = $props();
</script>

<div bind:this={ref} class={cn(alertVariants({ variant }), className)} {...restProps} role="alert">
	{@render children?.()}
</div>
