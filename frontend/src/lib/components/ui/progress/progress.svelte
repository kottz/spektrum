<script lang="ts">
	import { Progress as ProgressPrimitive } from 'bits-ui';
	import { cn, type WithElementRef } from '$lib/utils.js';

	type ProgressProps = WithElementRef<ProgressPrimitive.RootProps>;

	let {
		class: className,
		ref = $bindable(null),
		value = 0,
		max = 100,
		min = 0,
		...restProps
	}: ProgressProps = $props();

	const clampedMax = $derived(max ?? 100);
	const clampedMin = $derived(min ?? 0);
	const range = $derived(Math.max(clampedMax - clampedMin, 1));
	const normalized = $derived(
		value == null ? clampedMin : Math.min(Math.max(value, clampedMin), clampedMax)
	);
	const progress = $derived(((normalized - clampedMin) / range) * 100);
</script>

<ProgressPrimitive.Root
	bind:ref
	class={cn('bg-secondary relative h-4 w-full overflow-hidden rounded-full', className)}
	{value}
	{max}
	{min}
	{...restProps}
>
	<div
		class="bg-primary h-full w-full flex-1 transition-all"
		style={`transform: translateX(-${100 - progress}%)`}
	></div>
</ProgressPrimitive.Root>
