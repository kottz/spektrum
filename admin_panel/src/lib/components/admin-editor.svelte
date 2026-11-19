<script lang="ts">
	import Sidebar from '#lib/components/sidebar.svelte';
	import MediaTable from '#lib/components/media-table.svelte';
	import QuestionsTable from '#lib/components/questions-table.svelte';
	import SetsTable from '#lib/components/sets-table.svelte';
	import { adminStore } from '#lib/stores/data-manager.svelte.ts';
	import ChangesReview from './changes-review.svelte';
	import CharacterBank from './character-bank.svelte';
	import AuthComponent from '#lib/components/auth-component.svelte';

	type TabType = 'media' | 'characters' | 'questions' | 'sets';
	const state = $state({
		activeTab: 'media' as TabType
	});

	function setActiveTab(tab: TabType) {
		state.activeTab = tab;
	}
</script>

<div class="flex h-screen w-full">
	<Sidebar onTabChange={setActiveTab} activeTab={state.activeTab} />

	<div class="min-w-0 flex-1 border-x">
		{#if !adminStore.isLoaded()}
			<div class="p-6">
				<AuthComponent />
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

	<div class="w-96 border-l p-4">
		<ChangesReview />
	</div>
</div>
