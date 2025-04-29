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

	function handleInput(event: Event & { currentTarget: HTMLInputElement }) {
		onChange(event.currentTarget.value); // Notify parent of potential changes
	}

	function handleBlur(event: FocusEvent & { currentTarget: HTMLInputElement }) {
		const currentValue = event.currentTarget.value;
		if (currentValue !== value) {
			onCommit(currentValue);
		}
	}

	function handleKeyDown(event: KeyboardEvent & { currentTarget: HTMLInputElement }) {
		const currentValue = event.currentTarget.value;

		if (event.key === 'Enter') {
			event.preventDefault(); // Prevent default form submission
			if (currentValue !== value) {
				onCommit(currentValue);
			}
			event.currentTarget.blur();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			event.currentTarget.value = value;
			onChange(value);
			event.currentTarget.blur();
		}
	}
</script>

<Input
	{type}
	{value}
	{placeholder}
	on:input={handleInput}
	on:blur={handleBlur}
	on:keydown={handleKeyDown}
/>
