<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { Input } from '#lib/components/ui/input/index.ts';

	interface Props extends Omit<HTMLInputAttributes, 'value' | 'type' | 'files'> {
		value: string;
		type?: 'text' | 'number';
		onCommit?: (value: string) => void;
		onChange?: (value: string) => void;
	}

	let { value, type = 'text', onCommit, onChange, ...restProps }: Props = $props();

	function handleInput(event: Event & { currentTarget: HTMLInputElement }) {
		onChange?.(event.currentTarget.value);
	}

	function handleBlur(event: FocusEvent & { currentTarget: HTMLInputElement }) {
		const currentValue = event.currentTarget.value;
		if (currentValue !== value) {
			onCommit?.(currentValue);
		}
	}

	function handleKeyDown(event: KeyboardEvent & { currentTarget: HTMLInputElement }) {
		if (event.key === 'Enter') {
			event.preventDefault();
			// Blurring commits through handleBlur
			event.currentTarget.blur();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			event.currentTarget.value = value;
			onChange?.(value);
			event.currentTarget.blur();
		}
	}
</script>

<Input
	{...restProps}
	{type}
	{value}
	oninput={handleInput}
	onblur={handleBlur}
	onkeydown={handleKeyDown}
/>
