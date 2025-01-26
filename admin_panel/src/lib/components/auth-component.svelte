<script lang="ts">
	import { PUBLIC_DEV_ADMIN_PASSWORD, PUBLIC_SPEKTRUM_SERVER_URL } from '$env/static/public';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';

	const isDev = import.meta.env.DEV;
	const state = $state({
		password: '',
		isSubmitting: false,
		error: null as string | null
	});

	async function fetchData(authPassword: string) {
		try {
			state.isSubmitting = true;
			state.error = null;

			const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}/api/questions`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ password: authPassword })
			});

			if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
			const data = await response.json();
			adminStore.setData(data);
		} catch (error) {
			state.error = error instanceof Error ? error.message : 'Failed to load data';
		} finally {
			state.isSubmitting = false;
		}
	}

	function handleSubmit(e: Event) {
		e.preventDefault();
		fetchData(state.password);
	}

	if (isDev) {
		fetchData(PUBLIC_DEV_ADMIN_PASSWORD);
	}
</script>

{#if !isDev && adminStore.getState().media.length === 0}
	<Card>
		<CardHeader>
			<CardTitle>Authentication Required</CardTitle>
		</CardHeader>
		<CardContent>
			<form onsubmit={handleSubmit} class="space-y-4">
				<Input type="password" bind:value={state.password} placeholder="Enter admin password" />
				{#if state.error}
					<div class="text-sm text-red-500">{state.error}</div>
				{/if}
				<Button type="submit" class="w-full" disabled={state.isSubmitting}>
					{state.isSubmitting ? 'Loading...' : 'Submit'}
				</Button>
			</form>
		</CardContent>
	</Card>
{/if}
