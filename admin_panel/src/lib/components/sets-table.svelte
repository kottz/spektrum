<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import type { QuestionOption, QuestionSet, Question } from '$lib/types';
	import TableContainer from './table/table-container.svelte';
	import SearchInput from './table/search-input.svelte';
	import Pagination from './table/pagination.svelte';
	import EditableInput from './table/editable-input.svelte';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Separator } from '$lib/components/ui/separator';

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
		} as Partial<QuestionSet>,
		editingSet: null as QuestionSet | null,
		questionSearch: '',
		selectedQuestionIds: new Set<number>(),
		bulkActionState: 'none' as 'none' | 'select' | 'deselect',
		splitView: false,
		selectedSearch: '',
		unselectedSearch: ''
	});

	// Derived state
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

	// Question management derived state
	const allQuestions = $derived(adminStore.getState().questions);
	const mediaMap = $derived(new Map(adminStore.getState().media.map((m) => [m.id, m])));

	const optionsByQuestionId = $derived(
		adminStore.getState().options.reduce((map, opt) => {
			map.set(opt.question_id, [...(map.get(opt.question_id) || []), opt]);
			return map;
		}, new Map<number, QuestionOption[]>())
	);

	const filteredQuestions = $derived(() => {
		if (!state.editingSet) return [];
		const searchLower = state.questionSearch.toLowerCase();
		return allQuestions.filter((q) => {
			const media = mediaMap.get(q.media_id);
			const questionOptions = optionsByQuestionId.get(q.id) || [];
			return (
				q.id.toString().includes(state.questionSearch) ||
				q.question_text?.toLowerCase().includes(searchLower) ||
				media?.title.toLowerCase().includes(searchLower) ||
				questionOptions.some((opt) => opt.option_text.toLowerCase().includes(searchLower))
			);
		});
	});

	const allFilteredSelected = $derived(
		filteredQuestions().every((q) => state.selectedQuestionIds.has(q.id))
	);

	const allSelectedQuestions = $derived(
		allQuestions.filter((q) => state.selectedQuestionIds.has(q.id))
	);
	const allUnselectedQuestions = $derived(
		allQuestions.filter((q) => !state.selectedQuestionIds.has(q.id))
	);

	const filteredSelected = $derived(
		allSelectedQuestions.filter((q) => matchesSearch(q, state.selectedSearch))
	);
	const filteredUnselected = $derived(
		allUnselectedQuestions.filter((q) => matchesSearch(q, state.unselectedSearch))
	);

	function matchesSearch(q: Question, searchTerm: string): boolean {
		const searchLower = searchTerm.toLowerCase();
		const media = mediaMap.get(q.media_id);
		const questionOptions = optionsByQuestionId.get(q.id) || [];
		return (
			q.id.toString().includes(searchTerm) ||
			q.question_text?.toLowerCase().includes(searchLower) ||
			media?.title.toLowerCase().includes(searchLower) ||
			questionOptions.some((opt) => opt.option_text.toLowerCase().includes(searchLower))
		);
	}

	function getQuestionDetails(questionIds: number[]) {
		return questionIds
			.map((id) => {
				const question = allQuestions.find((q) => q.id === id);
				if (!question) return null;
				const media = mediaMap.get(question.media_id);
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

	function startEditSet(set: QuestionSet) {
		state.editingSet = { ...set };
		state.selectedQuestionIds = new Set(set.question_ids);
		state.questionSearch = '';
	}

	function cancelEdit() {
		state.editingSet = null;
		state.selectedQuestionIds.clear();
		state.splitView = false;
		state.selectedSearch = '';
		state.unselectedSearch = '';
	}

	function saveSetQuestions() {
		if (!state.editingSet) return;

		const newQuestionIds = Array.from(state.selectedQuestionIds);
		adminStore.modifyEntity('sets', state.editingSet.id, {
			question_ids: newQuestionIds
		});
		cancelEdit();
	}

	function toggleQuestionSelection(id: number) {
		const newSelection = new Set(state.selectedQuestionIds);
		newSelection.has(id) ? newSelection.delete(id) : newSelection.add(id);
		state.selectedQuestionIds = newSelection;
	}

	function toggleAllFiltered() {
		const newSelection = new Set(state.selectedQuestionIds);
		const currentlyAllSelected = filteredQuestions().every((q) => newSelection.has(q.id));

		filteredQuestions().forEach((q) => {
			currentlyAllSelected ? newSelection.delete(q.id) : newSelection.add(q.id);
		});

		state.selectedQuestionIds = newSelection;
	}

	function handleManageQuestions(set: QuestionSet) {
		startEditSet(set);
	}

	function deselectAllFiltered() {
		const newSelection = new Set(state.selectedQuestionIds);
		filteredSelected.forEach((q) => newSelection.delete(q.id));
		state.selectedQuestionIds = newSelection;
	}

	function selectAllFiltered() {
		const newSelection = new Set(state.selectedQuestionIds);
		filteredUnselected.forEach((q) => newSelection.add(q.id));
		state.selectedQuestionIds = newSelection;
	}
</script>

<TableContainer>
	<svelte:fragment slot="header-left">
		{#if state.editingSet}
			<div class="flex items-center gap-2">
				<Button variant="outline" size="sm" on:click={cancelEdit}>← Back</Button>
				{#if !state.splitView}
					<SearchInput
						value={state.questionSearch}
						placeholder="Search questions..."
						onInput={(value) => (state.questionSearch = value)}
					/>
				{/if}
				<Button variant="outline" size="sm" on:click={() => (state.splitView = !state.splitView)}>
					{state.splitView ? 'Single View' : 'Split View'}
				</Button>
			</div>
		{:else}
			<SearchInput
				value={state.searchTerm}
				placeholder="Search sets..."
				onInput={(value) => {
					state.searchTerm = value;
					state.currentPage = 0;
				}}
			/>
		{/if}
	</svelte:fragment>

	<svelte:fragment slot="header-right">
		{#if state.editingSet}
			<Button on:click={saveSetQuestions}>Save Changes</Button>
		{:else}
			<div class="flex gap-2">
				<Button on:click={handleAddSet}>Add Set</Button>
				<Button
					variant="outline"
					disabled={!adminStore.canUndo()}
					on:click={() => adminStore.undo()}
				>
					Undo
				</Button>
				<Button
					variant="outline"
					disabled={!adminStore.canRedo()}
					on:click={() => adminStore.redo()}
				>
					Redo
				</Button>
			</div>
		{/if}
	</svelte:fragment>

	{#if state.editingSet}
		{#if state.splitView}
			<div class="grid grid-cols-2 gap-4">
				<div>
					<div class="mb-2">
						<SearchInput
							value={state.selectedSearch}
							placeholder="Search selected..."
							onInput={(value) => (state.selectedSearch = value)}
						/>
					</div>
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head class="h-8 w-8">
									<Button
										variant="outline"
										size="icon"
										class="h-7 w-7"
										on:click={deselectAllFiltered}
										disabled={filteredSelected.length === 0}
									>
										→
									</Button>
								</Table.Head>
								<Table.Head>ID</Table.Head>
								<Table.Head>Media</Table.Head>
								<Table.Head>Type</Table.Head>
								<Table.Head>Options</Table.Head>
								<Table.Head>Question</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each filteredSelected as question}
								<Table.Row>
									<Table.Cell class="h-14">
										<Checkbox
											checked={state.selectedQuestionIds.has(question.id)}
											onCheckedChange={() => toggleQuestionSelection(question.id)}
										/>
									</Table.Cell>
									<Table.Cell>{question.id}</Table.Cell>
									<Table.Cell>
										{mediaMap.get(question.media_id)?.title || 'Unknown media'}
									</Table.Cell>
									<Table.Cell>{question.question_type}</Table.Cell>
									<Table.Cell>
										{optionsByQuestionId
											.get(question.id)
											?.map((opt) => opt.option_text)
											.join(', ')}
									</Table.Cell>
									<Table.Cell>
										{question.question_text || 'N/A'}
										{#if question.image_url}
											<span class="ml-2 text-xs text-gray-500">(Image)</span>
										{/if}
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</div>

				<div>
					<div class="mb-2">
						<SearchInput
							value={state.unselectedSearch}
							placeholder="Search unselected..."
							onInput={(value) => (state.unselectedSearch = value)}
						/>
					</div>
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head class="h-8 w-8">
									<Button
										variant="outline"
										size="icon"
										class="h-7 w-7"
										on:click={selectAllFiltered}
										disabled={filteredUnselected.length === 0}
									>
										←
									</Button>
								</Table.Head>
								<Table.Head>ID</Table.Head>
								<Table.Head>Media</Table.Head>
								<Table.Head>Type</Table.Head>
								<Table.Head>Options</Table.Head>
								<Table.Head>Question</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each filteredUnselected as question}
								<Table.Row>
									<Table.Cell class="h-14">
										<Checkbox
											checked={state.selectedQuestionIds.has(question.id)}
											onCheckedChange={() => toggleQuestionSelection(question.id)}
										/>
									</Table.Cell>
									<Table.Cell>{question.id}</Table.Cell>
									<Table.Cell>
										{mediaMap.get(question.media_id)?.title || 'Unknown media'}
									</Table.Cell>
									<Table.Cell>{question.question_type}</Table.Cell>
									<Table.Cell>
										{optionsByQuestionId
											.get(question.id)
											?.map((opt) => opt.option_text)
											.join(', ')}
									</Table.Cell>
									<Table.Cell>
										{question.question_text || 'N/A'}
										{#if question.image_url}
											<span class="ml-2 text-xs text-gray-500">(Image)</span>
										{/if}
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</div>
			</div>
		{:else}
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head class="h-8 w-8">
							<Checkbox checked={allFilteredSelected} onCheckedChange={toggleAllFiltered} />
						</Table.Head>
						<Table.Head>ID</Table.Head>
						<Table.Head>Media</Table.Head>
						<Table.Head>Type</Table.Head>
						<Table.Head>Options</Table.Head>
						<Table.Head>Question</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each filteredQuestions() as question}
						<Table.Row>
							<Table.Cell class="h-14">
								<Checkbox
									checked={state.selectedQuestionIds.has(question.id)}
									onCheckedChange={() => toggleQuestionSelection(question.id)}
								/>
							</Table.Cell>
							<Table.Cell>{question.id}</Table.Cell>
							<Table.Cell>
								{mediaMap.get(question.media_id)?.title || 'Unknown media'}
							</Table.Cell>
							<Table.Cell>{question.question_type}</Table.Cell>
							<Table.Cell>
								{optionsByQuestionId
									.get(question.id)
									?.map((opt) => opt.option_text)
									.join(', ')}
							</Table.Cell>
							<Table.Cell>
								{question.question_text || 'N/A'}
								{#if question.image_url}
									<span class="ml-2 text-xs text-gray-500">(Image)</span>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		{/if}
	{:else}
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
					<Table.Row class="bg-blue-50 dark:bg-gray-800">
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
									class="text-red-600 hover:bg-red-50 hover:dark:bg-red-800"
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
									class="text-red-600 hover:bg-red-50 hover:dark:bg-red-800"
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
	{/if}

	{#if !state.editingSet}
		<Pagination
			currentPage={state.currentPage}
			{totalPages}
			totalItems={filteredData().length}
			itemsPerPage={state.itemsPerPage}
			onPageChange={(page) => (state.currentPage = page)}
		/>
	{/if}
</TableContainer>
