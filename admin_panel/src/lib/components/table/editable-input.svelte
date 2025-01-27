<script lang="ts">
	import { Input } from '$lib/components/ui/input';

	let {
		value = '',
		type = 'text',
		placeholder = '',
		onCommit = (value: string) => {},
		onChange = (value: string) => {}
	} = $props<{
		value: string;
		type?: string;
		placeholder?: string;
		onCommit?: (value: string) => void;
		onChange?: (value: string) => void;
	}>();

	function handleKeyDown(event: KeyboardEvent & { currentTarget: HTMLInputElement }) {
		if (event.key === 'Enter') {
			onCommit(event.currentTarget.value);
			event.currentTarget.blur();
		}
	}
</script>

<Input
	{type}
	{value}
	{placeholder}
	on:input={(e) => onChange(e.currentTarget.value)}
	on:blur={() => onCommit(value)}
	on:keydown={handleKeyDown}
/>
