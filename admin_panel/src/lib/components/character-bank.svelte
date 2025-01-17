<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/admin-data';
	import { QuestionType } from '$lib/types';

	export let show = false;
	let searchTerm = '';

	// Get unique characters from question options, but only from character-type questions
	$: characterQuestions = $adminStore.questions.filter(
		(q) => q.question_type.toLowerCase() === QuestionType.Character
	);
	$: characterQuestionIds = new Set(characterQuestions.map((q) => q.id));
	$: distinctCharacters = [
		...new Set(
			$adminStore.options
				.filter((opt) => characterQuestionIds.has(opt.question_id) && opt.option_text)
				.map((opt) => opt.option_text)
		)
	];

	// Filter characters based on search term
	$: filteredCharacters = distinctCharacters.filter((char) =>
		char.toLowerCase().includes(searchTerm.toLowerCase())
	);
</script>

{#if show}
	<div class="fixed right-0 top-16 h-[calc(100vh-4rem)] w-96 border-l bg-white shadow-lg">
		<!-- Fixed header -->
		<div class="sticky top-0 space-y-4 border-b bg-white p-4">
			<div class="flex items-center justify-between">
				<h2 class="text-lg font-semibold">Character Bank</h2>
				<Button variant="outline" on:click={() => (show = false)}>Close</Button>
			</div>
			<Input type="text" placeholder="Search characters..." bind:value={searchTerm} />
		</div>
		<!-- Scrollable content -->
		<div class="h-[calc(100%-7rem)] overflow-y-auto p-4">
			<div class="grid grid-cols-4 gap-4">
				{#each filteredCharacters as char}
					<div
						class="cursor-grab transition-transform hover:scale-105"
						draggable="true"
						on:dragstart={(e) => {
							e.dataTransfer?.setData('text/plain', char);
							e.currentTarget.classList.add('dragging');
						}}
						on:dragend={(e) => {
							e.currentTarget.classList.remove('dragging');
						}}
					>
						<img src={`/img/${char}.avif`} alt={char} class="w-full rounded-lg" />
						<div class="mt-1 truncate text-center text-sm" title={char}>{char}</div>
					</div>
				{/each}
			</div>
			{#if filteredCharacters.length === 0}
				<div class="mt-4 text-center text-gray-500">No characters found</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.dragging {
		opacity: 0.5;
	}
</style>
