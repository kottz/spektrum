<script lang="ts">
	import { Separator as SeparatorPrimitive } from 'bits-ui';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type SeparatorProps = WithElementRef<SeparatorPrimitive.RootProps>;

	let {
		class: className,
		ref = $bindable(null),
		orientation = 'horizontal',
		decorative = false,
		...restProps
	}: SeparatorProps = $props();
</script>

<SeparatorPrimitive.Root
	bind:ref
	class={cn(
		'bg-border shrink-0',
		orientation === 'horizontal' ? 'h-px w-full' : 'min-h-full w-px',
		className
	)}
	{orientation}
	{decorative}
	{...restProps}
/>
