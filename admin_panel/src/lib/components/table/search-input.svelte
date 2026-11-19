<script lang="ts">
	import { Input } from '#lib/components/ui/input/index.ts';

	interface Props {
		value: string;
		placeholder?: string;
		onInput: (value: string) => void;
	}

	let { value, placeholder = 'Search...', onInput }: Props = $props();
</script>

<Input
	type="text"
	{placeholder}
	{value}
	oninput={(e) => onInput(e.currentTarget.value)}
	class="max-w-sm"
/>
