<script lang="ts">
	import { Progress as ProgressPrimitive } from 'bits-ui';
	import { cn } from '$lib/utils.js';

	type $$Props = ProgressPrimitive.RootProps;

	let className: $$Props['class'] = undefined;
	export { className as class };
	export let value: $$Props['value'] = 0;
	export let max: $$Props['max'] = 100;
	export let min: $$Props['min'] = 0;

	$: clampedMax = max ?? 100;
	$: clampedMin = min ?? 0;
	$: range = Math.max(clampedMax - clampedMin, 1);
	$: normalized = value == null ? clampedMin : Math.min(Math.max(value, clampedMin), clampedMax);
	$: progress = ((normalized - clampedMin) / range) * 100;
</script>

<ProgressPrimitive.Root
	class={cn('bg-secondary relative h-4 w-full overflow-hidden rounded-full', className)}
	{value}
	{max}
	{min}
	{...$$restProps}
>
	<div
		class="bg-primary h-full w-full flex-1 transition-all"
		style={`transform: translateX(-${100 - progress}%)`}
	></div>
</ProgressPrimitive.Root>
