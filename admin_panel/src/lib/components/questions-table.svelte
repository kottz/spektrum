<script lang="ts">
	import type { Character } from '$lib/types';
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as Command from '$lib/components/ui/command';
	import { adminStore } from '$lib/stores/data-manager.svelte';
	import type { Question, QuestionOption } from '$lib/types';
	import { QuestionType } from '$lib/types';
	import CharacterBank from '$lib/components/character-bank.svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Check, ChevronsUpDown } from 'lucide-svelte';
	import { Color } from '$lib/types';
	import { cn } from '$lib/utils';

	// State using runes
	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: '',
		showCharacterBank: false,
		selectedTypes: new Set(Object.values(QuestionType)),
		mediaSearchTerm: '',
		isAddingQuestion: false,
		newQuestionData: {
			id: 0,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		},
		tempOptions: [] as QuestionOption[]
	});

	const mediaById = $derived(new Map(adminStore.getState().media.map((m) => [m.id, m])));
	const optionsByQuestionId = $derived(
		adminStore.getState().options.reduce((map, opt) => {
			map.set(opt.question_id, [...(map.get(opt.question_id) || []), opt]);
			return map;
		}, new Map<number, QuestionOption[]>())
	);

	const filteredData = $derived(() => {
		const searchLower = state.searchTerm.toLowerCase();
		return adminStore.getState().questions.filter((question) => {
			if (!state.selectedTypes.has(question.question_type)) return false;

			const media = mediaById.get(question.media_id);
			const questionOptions = optionsByQuestionId.get(question.id) || [];

			return (
				question.id.toString().includes(state.searchTerm) ||
				question.question_text?.toLowerCase().includes(searchLower) ||
				media?.title.toLowerCase().includes(searchLower) ||
				questionOptions.some((opt) => opt.option_text.toLowerCase().includes(searchLower))
			);
		});
	});

	const filteredMediaOptions = $derived(() => {
		return adminStore
			.getState()
			.media.filter((media) => {
				if (!state.mediaSearchTerm) return true;
				const searchLower = state.mediaSearchTerm.toLowerCase();
				return (
					media.title?.toLowerCase().includes(searchLower) ||
					media.artist?.toLowerCase().includes(searchLower) ||
					media.id.toString().includes(searchLower)
				);
			})
			.slice(0, 5);
	});

	const totalPages = $derived(Math.ceil(filteredData().length / state.itemsPerPage));

	const paginatedData = $derived(() => {
		const currentFilteredData = filteredData(); // Call the filtered data function
		return currentFilteredData.slice(
			state.currentPage * state.itemsPerPage,
			(state.currentPage + 1) * state.itemsPerPage
		);
	});

	function getMediaTitle(mediaId: number): string {
		const media = adminStore.getState().media.find((m) => m.id === mediaId);
		return media?.title || 'Unknown Media';
	}

	function getQuestionOptions(questionId: number) {
		return adminStore.getState().options.filter((opt) => opt.question_id === questionId);
	}

	function handleDrop(event: DragEvent, questionId: number) {
		event.preventDefault();
		const charName = event.dataTransfer?.getData('text/plain');
		if (!charName) return;

		const newOption: QuestionOption = {
			id: Math.max(0, ...adminStore.getState().options.map((o: QuestionOption) => o.id)) + 1,
			question_id: questionId,
			option_text: charName,
			is_correct: false
		};

		adminStore.addEntity('options', newOption);
	}

	function removeOption(_questionId: number, optionId: number) {
		adminStore.deleteEntity('options', optionId);
	}

	function toggleCorrectOption(option: QuestionOption) {
		adminStore.modifyEntity('options', option.id, {
			...option,
			is_correct: !option.is_correct
		});
	}

	function toggleQuestionType(type: QuestionType) {
		// Create a new Set instance to trigger reactivity
		const newSet = new Set(state.selectedTypes);
		if (newSet.has(type)) {
			newSet.delete(type);
		} else {
			newSet.add(type);
		}
		state.selectedTypes = newSet; // Reassign to trigger reactivity
	}

	function toggleColorOption(color: Color) {
		state.tempOptions = state.tempOptions.some((o) => o.option_text === color)
			? state.tempOptions.filter((o) => o.option_text !== color)
			: [
					...state.tempOptions,
					{
						id: 0, // Temporary ID
						question_id: state.newQuestionData.id,
						option_text: color,
						is_correct: true
					}
				];
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

	function handleAddQuestion() {
		const maxId = Math.max(0, ...adminStore.getState().questions.map((q: Question) => q.id));
		state.newQuestionData = {
			id: maxId + 1,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		};
		state.isAddingQuestion = true;
	}

	function handleSaveQuestion() {
		try {
			adminStore.startBatch();

			// Add the question without options
			adminStore.addEntity('questions', {
				...state.newQuestionData,
				question_text: null
			});

			// Generate sequential IDs starting from current max
			const currentMaxId = Math.max(0, ...adminStore.getState().options.map((o) => o.id));
			let newOptionId = currentMaxId + 1;

			// Add all temp options with proper IDs
			state.tempOptions.forEach((option) => {
				adminStore.addEntity('options', {
					...option,
					id: newOptionId++,
					question_id: state.newQuestionData.id
				});
			});

			adminStore.commitBatch();
		} catch (error) {
			adminStore.cancelBatch();
			throw error;
		}

		// Reset state
		state.isAddingQuestion = false;
		state.newQuestionData = {
			id: 0,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		};
		state.tempOptions = [];
	}

	function handleCancelAdd() {
		// Cancel any ongoing batch operation
		adminStore.cancelBatch();

		// Reset local state
		state.isAddingQuestion = false;
		state.newQuestionData = {
			id: 0,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		};
	}

	function handleDeleteQuestion(questionId: number) {
		adminStore.deleteEntity('questions', questionId);
	}

	function handleEditQuestion(question: Question) {
		// TODO: Implement edit question functionality
		console.log('Edit question:', question);
	}
</script>

<div class="w-full">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-4">
			<Input
				type="text"
				placeholder="Search questions..."
				bind:value={state.searchTerm}
				oninput={() => (state.currentPage = 0)}
				class="max-w-sm"
			/>
			<Button
				variant="outline"
				on:click={() => (state.showCharacterBank = !state.showCharacterBank)}
			>
				Toggle Character Bank
			</Button>
		</div>
		<div class="flex gap-2">
			<Button on:click={handleAddQuestion}>Add Question</Button>
			<div class="flex gap-2">
				<Button variant="outline" disabled={!adminStore.canUndo} on:click={() => adminStore.undo()}>
					Undo
				</Button>
				<Button variant="outline" disabled={!adminStore.canRedo} on:click={() => adminStore.redo()}>
					Redo
				</Button>
			</div>
		</div>
	</div>

	<div class="rounded-md border">
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>ID</Table.Head>
					<Table.Head>Media</Table.Head>
					<Table.Head class="ml-0 pl-1">
						<DropdownMenu.Root>
							<DropdownMenu.Trigger asChild let:builder>
								<Button variant="outline" size="sm" builders={[builder]}>Type ↓</Button>
							</DropdownMenu.Trigger>
							<DropdownMenu.Content class="w-56">
								<DropdownMenu.Label>Question Types</DropdownMenu.Label>
								<DropdownMenu.Separator />
								{#each Object.values(QuestionType) as type}
									<DropdownMenu.CheckboxItem
										checked={state.selectedTypes.has(type)}
										onCheckedChange={() => toggleQuestionType(type)}
									>
										{type.charAt(0).toUpperCase() + type.slice(1)}
									</DropdownMenu.CheckboxItem>
								{/each}
							</DropdownMenu.Content>
						</DropdownMenu.Root>
					</Table.Head>
					<Table.Head>Question</Table.Head>
					<Table.Head>Options</Table.Head>
					<Table.Head>Status</Table.Head>
					<Table.Head class="text-right">Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#if state.isAddingQuestion}
					<Table.Row class="bg-blue-50">
						<Table.Cell>{state.newQuestionData.id}</Table.Cell>
						<Table.Cell>
							<Popover.Root let:ids>
								<Popover.Trigger asChild let:builder>
									<Button
										builders={[builder]}
										variant="outline"
										role="combobox"
										class="w-full justify-between"
									>
										{state.newQuestionData.media_id
											? getMediaTitle(state.newQuestionData.media_id)
											: 'Select media...'}
										<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
									</Button>
								</Popover.Trigger>
								<Popover.Content class="w-[300px] p-2">
									<Input
										type="text"
										placeholder="Search media..."
										bind:value={state.mediaSearchTerm}
										class="mb-2"
									/>
									<div class="max-h-[200px] overflow-y-auto">
										{#each filteredMediaOptions() as media}
											<div
												class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-gray-100"
												onclick={() => {
													state.newQuestionData.media_id = media.id;
													state.mediaSearchTerm = '';
												}}
											>
												<Check
													class={cn(
														'h-4 w-4',
														state.newQuestionData.media_id === media.id
															? 'text-blue-500'
															: 'text-transparent'
													)}
												/>
												<span>{media.title} - {media.artist}</span>
											</div>
										{/each}
										{#if filteredMediaOptions().length === 0}
											<div class="px-2 py-1.5 text-gray-500">No media found</div>
										{/if}
									</div>
								</Popover.Content>
							</Popover.Root>
						</Table.Cell>
						<Table.Cell>
							<Popover.Root let:ids>
								<Popover.Trigger asChild let:builder>
									<Button
										builders={[builder]}
										variant="outline"
										role="combobox"
										class="w-full justify-between"
									>
										{state.newQuestionData.question_type || 'Select type...'}
										<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
									</Button>
								</Popover.Trigger>
								<Popover.Content class="w-[200px] p-0">
									<Command.Root>
										<Command.Group>
											{#each Object.values(QuestionType) as type}
												<Command.Item
													value={type}
													onSelect={() => {
														state.newQuestionData.question_type = type;
													}}
												>
													<Check
														class={cn(
															'mr-2 h-4 w-4',
															state.newQuestionData.question_type !== type && 'text-transparent'
														)}
													/>
													{type}
												</Command.Item>
											{/each}
										</Command.Group>
									</Command.Root>
								</Popover.Content>
							</Popover.Root>
						</Table.Cell>
						<Table.Cell>
							<Input
								type="text"
								placeholder="Question text..."
								bind:value={state.newQuestionData.question_text}
							/>
						</Table.Cell>
						<Table.Cell class="w-[400px] min-w-[200px]">
							{#if state.newQuestionData.question_type === QuestionType.Color}
								<Popover.Root let:ids>
									<Popover.Trigger asChild let:builder>
										<Button builders={[builder]} variant="outline" class="w-full justify-between">
											Select colors...
											<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
										</Button>
									</Popover.Trigger>
									<Popover.Content class="w-[200px] p-0">
										<Command.Root>
											<Command.Group>
												{#each Object.values(Color) as color}
													<Command.Item value={color} onSelect={() => toggleColorOption(color)}>
														<Check
															class={cn(
																'mr-2 h-4 w-4',
																state.tempOptions.some((opt) => opt.option_text === color)
																	? 'opacity-100'
																	: 'opacity-0'
															)}
														/>
														{color}
													</Command.Item>
												{/each}
											</Command.Group>
										</Command.Root>
									</Popover.Content>
								</Popover.Root>
							{:else if state.newQuestionData.question_type === QuestionType.Character}
								<div
									class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
									ondragover={(e) => e.preventDefault()}
									ondrop={(e) => handleDrop(e, state.newQuestionData.id)}
								>
									{#each getQuestionOptions(state.newQuestionData.id) as option}
										{@const character = adminStore
											.getState()
											.characters.find((c: Character) => c.name === option.option_text)}
										<div class="group relative">
											<div
												class="flex cursor-pointer flex-col items-center"
												onclick={() => toggleCorrectOption(option)}
											>
												<img
													src={character ? character.image_url : `/img/${option.option_text}.avif`}
													alt={option.option_text}
													class="h-12 w-12 rounded transition-transform hover:scale-105"
													class:ring-2={option.is_correct}
													class:ring-green-500={option.is_correct}
												/>
												<span class="mt-1 w-12 truncate text-center text-xs">
													{option.option_text}
												</span>
											</div>
											<button
												class="absolute -right-2 -top-2 hidden h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white group-hover:flex"
												onclick={() => removeOption(state.newQuestionData.id, option.id)}
											>
												×
											</button>
										</div>
									{/each}
								</div>
							{/if}
						</Table.Cell>
						<Table.Cell>
							<Switch bind:checked={state.newQuestionData.is_active} />
						</Table.Cell>
						<Table.Cell class="text-right">
							<div class="flex justify-end gap-2">
								<Button variant="outline" size="sm" on:click={handleSaveQuestion}>Save</Button>
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
				{#each paginatedData() as question (question.id)}
					<Table.Row class="hover:bg-gray-50">
						<Table.Cell>{question.id}</Table.Cell>
						<Table.Cell>{getMediaTitle(question.media_id)}</Table.Cell>
						<Table.Cell>{question.question_type}</Table.Cell>
						<Table.Cell>
							{#if question.image_url}
								<div class="mb-2">
									<img
										src={question.image_url}
										alt="Question"
										class="h-12 w-12 rounded object-cover"
									/>
								</div>
							{/if}
							{question.question_text || 'N/A'}
						</Table.Cell>
						<Table.Cell class="w-[400px] min-w-[200px]">
							{#if question.question_type === QuestionType.Character}
								<div
									class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
									ondragover={(e) => e.preventDefault()}
									ondrop={(e) => handleDrop(e, question.id)}
								>
									{#each getQuestionOptions(question.id) as option}
										<div class="group relative">
											<div
												class="flex cursor-pointer flex-col items-center"
												onclick={() => toggleCorrectOption(option)}
											>
												<img
													src={`/img/${option.option_text}.avif`}
													alt={option.option_text}
													class="h-12 w-12 rounded transition-transform hover:scale-105"
													class:ring-2={option.is_correct}
													class:ring-green-500={option.is_correct}
												/>
												<span
													class="mt-1 w-12 truncate text-center text-xs"
													title={option.option_text}
													class:text-green-600={option.is_correct}
												>
													{option.option_text}
												</span>
											</div>
											<button
												class="absolute -right-2 -top-2 hidden h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white group-hover:flex"
												onclick={(e) => {
													e.stopPropagation();
													removeOption(question.id, option.id);
												}}
											>
												×
											</button>
										</div>
									{/each}
								</div>
							{:else}
								<div class="flex flex-col gap-1">
									{#each getQuestionOptions(question.id) as option}
										<div class:text-green-600={option.is_correct}>
											{option.option_text}
										</div>
									{/each}
								</div>
							{/if}
						</Table.Cell>
						<Table.Cell>
							<span
								class={`inline-flex rounded-full px-2 py-1 text-xs font-semibold ${
									question.is_active ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
								}`}
							>
								{question.is_active ? 'Active' : 'Inactive'}
							</span>
						</Table.Cell>
						<Table.Cell class="text-right">
							<div class="flex justify-end gap-2">
								<Button variant="outline" size="sm" on:click={() => handleEditQuestion(question)}>
									Edit
								</Button>
								<Button
									variant="outline"
									size="sm"
									class="text-red-600 hover:bg-red-50"
									on:click={() => handleDeleteQuestion(question.id)}
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
			)} of {filteredData().length} questions
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

<CharacterBank bind:show={state.showCharacterBank} />
