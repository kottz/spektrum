<script lang="ts">
	import Sidebar from '$lib/components/sidebar.svelte';
	import MediaTable from '$lib/components/media-table.svelte';
	import QuestionsTable from '$lib/components/questions-table.svelte';
	import SetsTable from '$lib/components/sets-table.svelte';
	import { activeTab, adminStore } from '$lib/stores/admin-data';
	import { onMount } from 'svelte';

	onMount(async () => {
		try {
			adminStore.setLoading(true);
			const response = await fetch('http://localhost:8765/api/questions', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					password: 'test123'
				})
			});

			if (!response.ok) {
				throw new Error(`HTTP error! status: ${response.status}`);
			}

			const data = await response.json();
			adminStore.setData(data);
		} catch (error) {
			adminStore.setError(error instanceof Error ? error.message : 'Failed to load data');
		} finally {
			adminStore.setLoading(false);
		}
	});
</script>

<div class="flex h-screen w-full">
	<Sidebar />

	<main class="flex-1 overflow-auto p-6">
		{#if $adminStore.isLoading}
			<div class="flex h-full items-center justify-center">
				<div class="text-lg">Loading...</div>
			</div>
		{:else if $adminStore.error}
			<div class="rounded-md bg-red-50 p-4 text-red-700">
				{$adminStore.error}
			</div>
		{:else if $activeTab === 'media'}
			<MediaTable />
		{:else if $activeTab === 'questions'}
			<QuestionsTable />
		{:else if $activeTab === 'sets'}
			<SetsTable />
		{/if}
	</main>
</div>
