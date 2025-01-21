<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import type { QuestionSet } from '$lib/types';

	// State with runes
	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: ''
	});

	// Derived values
	const filteredData = $derived(() => {
		const sets = adminStore.getState().sets;
		return sets.filter(
			(set) =>
				set.name.toLowerCase().includes(state.searchTerm.toLowerCase()) ||
				set.id.toString().includes(state.searchTerm)
		);
	});

	const totalPages = $derived(Math.ceil(filteredData().length / state.itemsPerPage));

	const paginatedData = $derived(() => {
		return filteredData().slice(
			state.currentPage * state.itemsPerPage,
			(state.currentPage + 1) * state.itemsPerPage
		);
	});

	// Get question details for a set
	function getQuestionDetails(questionIds: number[]) {
		const storeState = adminStore.getState();
		return questionIds
			.map((id) => {
				const question = storeState.questions.find((q) => q.id === id);
				if (!question) return null;

				const media = storeState.media.find((m) => m.id === question.media_id);
				return {
					question,
					media
				};
			})
			.filter(Boolean);
	}

	function nextPage() {
		if (state.currentPage < totalPages - 1) {
			state.currentPage++;
		}
	}

	function previousPage() {
		if (state.currentPage > 0) {
			state.currentPage--;
		}
	}

	function handleAddSet() {
		const maxId = Math.max(0, ...adminStore.getState().sets.map((s) => s.id));
		const newSet: QuestionSet = {
			id: maxId + 1,
			name: 'New Set',
			question_ids: []
		};
		adminStore.addEntity('sets', newSet);
	}

	function handleEditSet(set: QuestionSet) {
		// TODO: Implement edit set functionality
		console.log('Edit set:', set);
	}

	function handleDeleteSet(setId: number) {
		adminStore.deleteEntity('sets', setId);
	}

	function handleManageQuestions(set: QuestionSet) {
		// TODO: Implement question management for set
		console.log('Manage questions for set:', set);
	}
</script>

<div class="w-full">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-4">
			<Input
				type="text"
				placeholder="Search sets..."
				bind:value={state.searchTerm}
				class="max-w-sm"
			/>
		</div>
		<div class="flex gap-2">
			<Button on:click={handleAddSet}>Add Set</Button>
			<Button variant="outline" disabled={!adminStore.canUndo()} on:click={() => adminStore.undo()}>
				Undo
			</Button>
			<Button variant="outline" disabled={!adminStore.canRedo()} on:click={() => adminStore.redo()}>
				Redo
			</Button>
		</div>
	</div>

	<div class="rounded-md border">
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>ID</Table.Head>
					<Table.Head>Name</Table.Head>
					<Table.Head>Questions</Table.Head>
					<Table.Head class="text-right">Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each paginatedData() as set}
					{@const questionDetails = getQuestionDetails(set.question_ids)}
					<Table.Row>
						<Table.Cell>{set.id}</Table.Cell>
						<Table.Cell class="font-medium">{set.name}</Table.Cell>
						<Table.Cell>
							<div class="flex flex-col gap-1">
								<div class="flex items-center gap-2">
									<span
										class="rounded-full bg-blue-100 px-2 py-1 text-xs font-semibold text-blue-800"
									>
										{set.question_ids.length} questions
									</span>
									<Button variant="outline" size="sm" on:click={() => handleManageQuestions(set)}>
										Manage
									</Button>
								</div>
								{#if questionDetails.length > 0}
									<div class="mt-2 text-sm text-gray-500">
										<details>
											<summary class="cursor-pointer">Show questions</summary>
											<div class="mt-2 space-y-1">
												{#each questionDetails as detail}
													{#if detail}
														<div class="flex items-center gap-2">
															<span class="font-medium">
																{detail.media?.title}:
															</span>
															{detail.question.question_text || 'No question text'}
														</div>
													{/if}
												{/each}
											</div>
										</details>
									</div>
								{/if}
							</div>
						</Table.Cell>
						<Table.Cell class="text-right">
							<div class="flex justify-end gap-2">
								<Button variant="outline" size="sm" on:click={() => handleEditSet(set)}>
									Edit
								</Button>
								<Button
									variant="outline"
									size="sm"
									class="text-red-600 hover:bg-red-50"
									on:click={() => handleDeleteSet(set.id)}
								>
									Delete
								</Button>
							</div>
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	</div>

	<div class="mt-4 flex items-center justify-between">
		<div class="text-sm text-muted-foreground">
			Showing {state.currentPage * state.itemsPerPage + 1} to {Math.min(
				(state.currentPage + 1) * state.itemsPerPage,
				filteredData().length
			)} of {filteredData().length} sets
		</div>
		<div class="flex gap-2">
			<Button
				variant="outline"
				size="sm"
				on:click={previousPage}
				disabled={state.currentPage === 0}
			>
				Previous
			</Button>
			<Button
				variant="outline"
				size="sm"
				on:click={nextPage}
				disabled={state.currentPage >= totalPages - 1}
			>
				Next
			</Button>
		</div>
	</div>
</div>
