<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	type $$Props = HTMLAttributes<HTMLDivElement>;

	let className: $$Props['class'] = undefined;
	export { className as class };
</script>

<div
	class={cn('rounded-lg border border-border bg-card text-card-foreground shadow', className)}
	{...$$restProps}
>
	<slot />
</div>
