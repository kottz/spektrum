<script lang="ts">
	import Sidebar from '$lib/components/sidebar.svelte';
	import MediaTable from '$lib/components/media-table.svelte';
	import QuestionsTable from '$lib/components/questions-table.svelte';
	import SetsTable from '$lib/components/sets-table.svelte';
	import { PUBLIC_DEV_ADMIN_PASSWORD } from '$env/static/public';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import { onMount } from 'svelte';
	import ChangesReview from './changes-review.svelte';
	import CharacterBank from './character-bank.svelte';

	type TabType = 'media' | 'characters' | 'questions' | 'sets';
	const state = $state({
		activeTab: 'media' as TabType
	});

	onMount(async () => {
		try {
			adminStore.setLoading(true);
			const response = await fetch('http://localhost:8765/api/questions', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({
					password: PUBLIC_DEV_ADMIN_PASSWORD
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

	function setActiveTab(tab: TabType) {
		state.activeTab = tab;
	}
</script>

<div class="flex h-screen w-full">
	<!-- Left Sidebar -->
	<Sidebar onTabChange={setActiveTab} activeTab={state.activeTab} />

	<!-- Main Content -->
	<div class="min-w-0 flex-1 border-x">
		{#if adminStore.isLoading()}
			<div class="flex h-full items-center justify-center">
				<div class="text-lg">Loading...</div>
			</div>
		{:else if adminStore.getError()}
			<div class="m-6 rounded-md bg-red-50 p-4 text-red-700">
				{adminStore.getError()}
			</div>
		{:else}
			<div class="h-full overflow-auto p-6">
				{#if state.activeTab === 'media'}
					<MediaTable />
				{:else if state.activeTab === 'characters'}
					<CharacterBank />
				{:else if state.activeTab === 'questions'}
					<QuestionsTable />
				{:else if state.activeTab === 'sets'}
					<SetsTable />
				{/if}
			</div>
		{/if}
	</div>

	<!-- Right Changes Panel -->
	<div class="w-96 border-l p-4">
		<ChangesReview />
	</div>
</div>
