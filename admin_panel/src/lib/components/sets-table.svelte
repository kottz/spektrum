<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import type { QuestionSet } from '$lib/types';
	import { cn } from '$lib/utils';
	import TableContainer from './table/table-container.svelte';
	import SearchInput from './table/search-input.svelte';
	import Pagination from './table/pagination.svelte';
	import EditableInput from './table/editable-input.svelte';

	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: '',
		isAddingSet: false,
		editingValues: new Map<string, string>(),
		newSetData: {
			id: 0,
			name: '',
			question_ids: []
		} as Partial<QuestionSet>
	});

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

	function getQuestionDetails(questionIds: number[]) {
		const storeState = adminStore.getState();
		return questionIds
			.map((id) => {
				const question = storeState.questions.find((q) => q.id === id);
				if (!question) return null;
				const media = storeState.media.find((m) => m.id === question.media_id);
				return { question, media };
			})
			.filter(Boolean);
	}

	function getEditKey(id: number, field: string): string {
		return `${id}-${field}`;
	}

	function getEditingValue(id: number, field: keyof QuestionSet, currentValue: string): string {
		const editKey = getEditKey(id, field);
		return state.editingValues.has(editKey) ? state.editingValues.get(editKey)! : currentValue;
	}

	function handleSetFieldChange(id: number, field: keyof QuestionSet, value: string) {
		const editKey = getEditKey(id, field);
		state.editingValues.set(editKey, value);
	}

	function commitChanges(id: number, field: keyof QuestionSet) {
		const editKey = getEditKey(id, field);
		const newValue = state.editingValues.get(editKey);

		if (newValue !== undefined) {
			adminStore.modifyEntity('sets', id, { [field]: newValue });
			state.editingValues.delete(editKey);
		}
	}

	function handleKeyDown(
		event: KeyboardEvent & { currentTarget: HTMLInputElement },
		id: number,
		field: keyof QuestionSet
	) {
		if (event.key === 'Enter') {
			commitChanges(id, field);
			event.currentTarget.blur();
		}
	}

	function handleAddSet() {
		const maxId = Math.max(0, ...adminStore.getState().sets.map((s) => s.id));
		state.newSetData = {
			id: maxId + 1,
			name: '',
			question_ids: []
		};
		state.isAddingSet = true;
	}

	function handleSaveSet() {
		if (state.newSetData.name) {
			adminStore.addEntity('sets', state.newSetData as QuestionSet);
			state.isAddingSet = false;
			state.newSetData = { id: 0, name: '', question_ids: [] };
		}
	}

	function handleCancelAdd() {
		state.isAddingSet = false;
		state.newSetData = { id: 0, name: '', question_ids: [] };
	}

	function handleDeleteSet(setId: number) {
		adminStore.deleteEntity('sets', setId);
	}

	function handleManageQuestions(set: QuestionSet) {
		// TODO: Implement question management
		console.log('Manage questions for set:', set);
	}
</script>

<TableContainer>
	<svelte:fragment slot="header-left">
		<SearchInput
			value={state.searchTerm}
			placeholder="Search sets..."
			onInput={(value) => {
				state.searchTerm = value;
				state.currentPage = 0;
			}}
		/>
	</svelte:fragment>

	<svelte:fragment slot="header-right">
		<div class="flex gap-2">
			<Button on:click={handleAddSet}>Add Set</Button>
			<Button variant="outline" disabled={!adminStore.canUndo()} on:click={() => adminStore.undo()}>
				Undo
			</Button>
			<Button variant="outline" disabled={!adminStore.canRedo()} on:click={() => adminStore.redo()}>
				Redo
			</Button>
		</div>
	</svelte:fragment>

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
			{#if state.isAddingSet}
				<Table.Row class="bg-blue-50">
					<Table.Cell>{state.newSetData.id}</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={state.newSetData.name || ''}
							placeholder="Name"
							onChange={(value) => (state.newSetData.name = value)}
							onCommit={(value) => (state.newSetData.name = value)}
						/>
					</Table.Cell>
					<Table.Cell>0 questions</Table.Cell>
					<Table.Cell class="text-right">
						<div class="flex justify-end gap-2">
							<Button variant="outline" size="sm" on:click={handleSaveSet}>Save</Button>
							<Button
								variant="outline"
								size="sm"
								class="text-red-600 hover:bg-red-50"
								on:click={handleCancelAdd}
							>
								Cancel
							</Button>
						</div>
					</Table.Cell>
				</Table.Row>
			{/if}

			{#each paginatedData() as set (set.id)}
				{@const questionDetails = getQuestionDetails(set.question_ids)}
				<Table.Row>
					<Table.Cell>{set.id}</Table.Cell>
					<Table.Cell>
						<EditableInput
							value={getEditingValue(set.id, 'name', set.name)}
							onChange={(value) => handleSetFieldChange(set.id, 'name', value)}
							onCommit={(value) => commitChanges(set.id, 'name')}
						/>
					</Table.Cell>
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
														<span class="font-medium">{detail.media?.title}:</span>
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

	<Pagination
		currentPage={state.currentPage}
		{totalPages}
		totalItems={filteredData().length}
		itemsPerPage={state.itemsPerPage}
		onPageChange={(page) => (state.currentPage = page)}
	/>
</TableContainer>
