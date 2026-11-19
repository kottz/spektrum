<script lang="ts">
	import { PUBLIC_DEV_ADMIN_PASSWORD } from '$app/env/public';
	import { adminStore } from '#lib/stores/data-manager.svelte.ts';
	import { Card, CardContent, CardHeader, CardTitle } from '#lib/components/ui/card/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { api, ApiError } from '#lib/api.ts';

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

			adminStore.setData(await api.getStoredData({ password: authPassword }));
		} catch (error) {
			if (error instanceof ApiError && error.status === 401) {
				state.error = 'Wrong password';
			} else {
				state.error = error instanceof Error ? error.message : 'Failed to load data';
			}
		} finally {
			state.isSubmitting = false;
		}
	}

	function handleSubmit(e: Event) {
		e.preventDefault();
		fetchData(state.password);
	}

	if (isDev && PUBLIC_DEV_ADMIN_PASSWORD) {
		fetchData(PUBLIC_DEV_ADMIN_PASSWORD);
	}
</script>

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
