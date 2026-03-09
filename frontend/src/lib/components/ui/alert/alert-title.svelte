<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import type { HeadingLevel } from './index.js';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type AlertTitleProps = WithElementRef<HTMLAttributes<HTMLHeadingElement>> & {
		level?: HeadingLevel;
		children?: Snippet;
	};

	let {
		class: className,
		level = 'h5',
		ref = $bindable(null),
		children,
		...restProps
	}: AlertTitleProps = $props();
</script>

<svelte:element
	this={level}
	bind:this={ref}
	class={cn('mb-1 leading-none font-medium tracking-tight', className)}
	{...restProps}
>
	{@render children?.()}
</svelte:element>
