<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import type { HeadingLevel } from './index.js';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type CardTitleProps = WithElementRef<HTMLAttributes<HTMLHeadingElement>> & {
		tag?: HeadingLevel;
		children?: Snippet;
	};

	let {
		class: className,
		tag = 'h3',
		ref = $bindable(null),
		children,
		...restProps
	}: CardTitleProps = $props();
</script>

<svelte:element
	this={tag}
	bind:this={ref}
	class={cn('text-lg leading-none font-semibold tracking-tight', className)}
	{...restProps}
>
	{@render children?.()}
</svelte:element>
