<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.ts';
	import LightSwitch from '#lib/components/light-switch.svelte';

	type TabType = 'media' | 'characters' | 'questions' | 'sets';

	interface Props {
		activeTab: TabType;
		onTabChange: (tab: TabType) => void;
	}

	const tabs = [
		{ id: 'media' as const, label: 'Media', symbol: '📊' },
		{ id: 'characters' as const, label: 'Characters', symbol: '🧙' },
		{ id: 'questions' as const, label: 'Questions', symbol: '❓' },
		{ id: 'sets' as const, label: 'Sets', symbol: '📁' }
	];

	let { activeTab, onTabChange }: Props = $props();
</script>

<aside class="w-48 border-r">
	<nav class="flex h-full flex-col p-4">
		<div class="mb-6 flex gap-1 text-xl font-bold">
			Admin Panel
			<LightSwitch />
		</div>
		<div class="space-y-2">
			{#each tabs as tab (tab.id)}
				<Button
					variant={activeTab === tab.id ? 'default' : 'ghost'}
					class="w-full justify-start"
					onclick={() => onTabChange(tab.id)}
				>
					<span class="mr-2">{tab.symbol}</span>
					{tab.label}
				</Button>
			{/each}
		</div>
	</nav>
</aside>
