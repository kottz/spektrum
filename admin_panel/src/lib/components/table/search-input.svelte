<script lang="ts">
	import { Input } from '$lib/components/ui/input';

	let {
		value = '',
		placeholder = 'Search...',
		onInput = (value: string) => {}
	} = $props<{
		value: string;
		placeholder?: string;
		onInput: (value: string) => void;
	}>();
</script>

<Input
	type="text"
	{placeholder}
	{value}
	on:input={(e) => onInput(e.currentTarget.value)}
	class="max-w-sm"
/>
