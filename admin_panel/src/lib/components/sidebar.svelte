<script lang="ts">
	import { activeTab } from '$lib/stores/admin-data';
	import { Button } from '$lib/components/ui/button';

	const tabs = [
		{ id: 'media' as const, label: 'Media', symbol: '📊' },
		{ id: 'questions' as const, label: 'Questions', symbol: '❓' },
		{ id: 'sets' as const, label: 'Sets', symbol: '📁' }
	];
</script>

<aside class="w-64 border-r bg-gray-50">
	<nav class="flex h-full flex-col p-4">
		<div class="mb-6 text-xl font-bold">Admin Panel</div>

		<div class="space-y-2">
			{#each tabs as tab}
				<Button
					variant={$activeTab === tab.id ? 'default' : 'ghost'}
					class="w-full justify-start"
					on:click={() => activeTab.set(tab.id)}
				>
					<span class="mr-2">{tab.symbol}</span>
					{tab.label}
				</Button>
			{/each}
		</div>
	</nav>
</aside>
