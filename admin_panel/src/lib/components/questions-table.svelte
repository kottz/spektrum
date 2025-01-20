<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as Command from '$lib/components/ui/command';
	import { adminStore } from '$lib/stores/admin-data';
	import type { Question, QuestionOption } from '$lib/types';
	import { QuestionType } from '$lib/types';
	import CharacterBank from '$lib/components/character-bank.svelte';
	import ChangesReview from '$lib/components/changes-review.svelte';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Check, ChevronsUpDown } from 'lucide-svelte';
	import { Color } from '$lib/types';
	import { cn } from '$lib/utils';

	// Pagination state
	let currentPage = 0;
	let itemsPerPage = 10;
	let searchTerm = '';
	let showCharacterBank = false;
	let selectedTypes = new Set(Object.values(QuestionType));

	// Filtered and paginated data
	$: filteredData = $adminStore.questions.filter((question) => {
		const searchLower = searchTerm.toLowerCase();
		currentPage = 0; // move to first page when searching

		// Check if question type is selected
		if (!selectedTypes.has(question.question_type)) {
			return false;
		}

		// Check question ID and text
		if (
			question.id.toString().includes(searchLower) ||
			question.question_text?.toLowerCase().includes(searchLower)
		) {
			return true;
		}

		// Check related media title
		const media = $adminStore.media.find((m) => m.id === question.media_id);
		if (media?.title.toLowerCase().includes(searchLower)) {
			return true;
		}

		// Check character names in options
		const questionOptions = $adminStore.options.filter((opt) => opt.question_id === question.id);
		return questionOptions.some((opt) => opt.option_text.toLowerCase().includes(searchLower));
	});

	// In add question mode for the media combobox
	let mediaSearchTerm = '';

	$: filteredMediaOptions = $adminStore.media
		.filter((media) => {
			if (!mediaSearchTerm) return true;
			const searchLower = mediaSearchTerm.toLowerCase();
			return (
				media.title?.toLowerCase().includes(searchLower) ||
				media.artist?.toLowerCase().includes(searchLower) ||
				media.id.toString().includes(searchLower)
			);
		})
		.slice(0, 5); // Only show top 5 matches

	$: totalPages = Math.ceil(filteredData.length / itemsPerPage);

	$: paginatedData = filteredData.slice(
		currentPage * itemsPerPage,
		(currentPage + 1) * itemsPerPage
	);

	// Get media title by ID
	function getMediaTitle(mediaId: number): string {
		const media = $adminStore.media.find((m) => m.id === mediaId);
		return media?.title || 'Unknown Media';
	}

	// Get question options
	function getQuestionOptions(questionId: number) {
		return $adminStore.options.filter((opt) => opt.question_id === questionId);
	}

	function handleDrop(event: DragEvent, questionId: number) {
		event.preventDefault();
		const charName = event.dataTransfer?.getData('text/plain');
		if (!charName) return;

		const newOption: QuestionOption = {
			id: Math.max(0, ...$adminStore.options.map((o) => o.id)) + 1,
			question_id: questionId,
			option_text: charName,
			is_correct: false
		};

		adminStore.addEntity('options', newOption);
	}

	function removeOption(questionId: number, optionId: number) {
		adminStore.markForDeletion('options', optionId);
	}

	function toggleCorrectOption(option: QuestionOption) {
		adminStore.modifyEntity('options', option.id, {
			...option,
			is_correct: !option.is_correct
		});
	}

	function toggleQuestionType(type: QuestionType) {
		if (selectedTypes.has(type)) {
			selectedTypes.delete(type);
		} else {
			selectedTypes.add(type);
		}
		selectedTypes = selectedTypes; // Trigger reactivity
	}

	function nextPage() {
		if (currentPage < totalPages - 1) {
			currentPage++;
		}
	}

	function previousPage() {
		if (currentPage > 0) {
			currentPage--;
		}
	}

	let isAddingQuestion = false;
	let newQuestionData = {
		id: 0,
		media_id: 0,
		question_type: '',
		question_text: '',
		image_url: null,
		is_active: true
	};

	function handleAddQuestion() {
		const maxId = Math.max(...$adminStore.questions.map((q) => q.id));
		newQuestionData.id = maxId + 1;
		isAddingQuestion = true;
	}

	function handleSaveQuestion() {
		adminStore.addEntity('questions', newQuestionData);
		isAddingQuestion = false;
		// Reset the form
		newQuestionData = {
			id: 0,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		};
	}

	function handleCancelAdd() {
		isAddingQuestion = false;
		// Reset the form
		newQuestionData = {
			id: 0,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		};
	}

	function handleEditQuestion(question: Question) {
		// TODO: Implement edit question functionality
		console.log('Edit question:', question);
	}

	// Add this to your component's state
	let questionsMarkedForDeletion = new Set<number>();

	// In your questions-table.svelte:
	function handleDeleteQuestion(questionId: number) {
		if (questionsMarkedForDeletion.has(questionId)) {
			adminStore.undoDelete('questions', questionId);
			questionsMarkedForDeletion.delete(questionId);
		} else {
			adminStore.markForDeletion('questions', questionId);
			questionsMarkedForDeletion.add(questionId);
		}
		questionsMarkedForDeletion = questionsMarkedForDeletion;
	}
</script>

<div class="w-full">
	<div class="mb-4 flex items-center justify-between">
		<div class="flex items-center gap-4">
			<Input
				type="text"
				placeholder="Search questions..."
				bind:value={searchTerm}
				class="max-w-sm"
			/>
			{#if $adminStore.questions.some((q) => q.question_type.toLowerCase() === QuestionType.Character)}
				<Button variant="outline" on:click={() => (showCharacterBank = !showCharacterBank)}>
					Toggle Character Bank
				</Button>
			{/if}
		</div>
		<div class="flex gap-2">
			<Button on:click={handleAddQuestion}>Add Question</Button>
			<pre class="text-xs">
				Pending Changes Length: {$adminStore.pendingChanges.length}
				Pending Changes: {JSON.stringify($adminStore.pendingChanges, null, 2)}
			</pre>
			{#if $adminStore.pendingChanges.length > 0}
				<ChangesReview />
			{/if}
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
										checked={selectedTypes.has(type)}
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
				{#if isAddingQuestion}
					<Table.Row class="bg-blue-50">
						<Table.Cell>{newQuestionData.id}</Table.Cell>
						<Table.Cell>
							<Popover.Root let:ids>
								<Popover.Trigger asChild let:builder>
									<Button
										builders={[builder]}
										variant="outline"
										role="combobox"
										class="w-full justify-between"
									>
										{newQuestionData.media_id
											? getMediaTitle(newQuestionData.media_id)
											: 'Select media...'}
										<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
									</Button>
								</Popover.Trigger>
								<Popover.Content class="w-[300px] p-2">
									<Input
										type="text"
										placeholder="Search media..."
										bind:value={mediaSearchTerm}
										class="mb-2"
										onInput={(e) => {
											mediaSearchTerm = e.currentTarget.value;
										}}
									/>
									<div class="max-h-[200px] overflow-y-auto">
										{#each filteredMediaOptions as media}
											<div
												class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-gray-100"
												on:click={() => {
													newQuestionData.media_id = media.id;
													mediaSearchTerm = '';
												}}
											>
												<Check
													class={cn(
														'h-4 w-4',
														newQuestionData.media_id === media.id
															? 'text-blue-500'
															: 'text-transparent'
													)}
												/>
												<span>{media.title} - {media.artist}</span>
											</div>
										{/each}
										{#if filteredMediaOptions.length === 0}
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
										{newQuestionData.question_type || 'Select type...'}
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
														newQuestionData.question_type = type;
													}}
												>
													<Check
														class={cn(
															'mr-2 h-4 w-4',
															newQuestionData.question_type !== type && 'text-transparent'
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
								bind:value={newQuestionData.question_text}
							/>
						</Table.Cell>
						<Table.Cell>
							{#if newQuestionData.question_type === QuestionType.Color}
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
													<Command.Item
														value={color}
														onSelect={() => {
															const option = {
																id: Math.max(0, ...$adminStore.options.map((o) => o.id)) + 1,
																question_id: newQuestionData.id,
																option_text: color,
																is_correct: false
															};
															adminStore.update((state) => ({
																...state,
																options: [...state.options, option]
															}));
														}}
													>
														<Check class="mr-2 h-4 w-4" />
														{color}
													</Command.Item>
												{/each}
											</Command.Group>
										</Command.Root>
									</Popover.Content>
								</Popover.Root>
							{:else if newQuestionData.question_type === QuestionType.Character}
								<div
									class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
									on:dragover|preventDefault
									on:drop={(e) => handleDrop(e, newQuestionData.id)}
								>
									{#each getQuestionOptions(newQuestionData.id) as option}
										<div class="group relative">
											<div
												class="flex cursor-pointer flex-col items-center"
												on:click={() => toggleCorrectOption(option)}
											>
												<img
													src={`/img/${option.option_text}.avif`}
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
												on:click={() => removeOption(newQuestionData.id, option.id)}
											>
												×
											</button>
										</div>
									{/each}
								</div>
							{/if}
						</Table.Cell>
						<Table.Cell>
							<Switch bind:checked={newQuestionData.is_active} />
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
				{#each paginatedData as question (question.id)}
					<Table.Row
						class={cn(
							'transition-colors',
							questionsMarkedForDeletion.has(question.id)
								? '!hover:bg-red-100 bg-red-100 hover:bg-red-100'
								: 'hover:bg-gray-50'
						)}
					>
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
						<Table.Cell>
							{#if question.question_type === QuestionType.Character}
								<div
									class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
									on:dragover|preventDefault
									on:drop={(e) => handleDrop(e, question.id)}
								>
									{#each getQuestionOptions(question.id) as option}
										<div class="group relative">
											<div
												class="flex cursor-pointer flex-col items-center"
												on:click={() => toggleCorrectOption(option)}
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
												on:click={() => removeOption(question.id, option.id)}
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
									class={questionsMarkedForDeletion.has(question.id)
										? 'text-green-600 hover:bg-green-50'
										: 'text-red-600 hover:bg-red-50'}
									on:click={() => handleDeleteQuestion(question.id)}
								>
									{questionsMarkedForDeletion.has(question.id) ? 'Undo' : 'Delete'}
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
			Showing {currentPage * itemsPerPage + 1} to {Math.min(
				(currentPage + 1) * itemsPerPage,
				filteredData.length
			)} of {filteredData.length} questions
		</div>
		<div class="flex gap-2">
			<Button variant="outline" size="sm" on:click={previousPage} disabled={currentPage === 0}>
				Previous
			</Button>
			<Button
				variant="outline"
				size="sm"
				on:click={nextPage}
				disabled={currentPage >= totalPages - 1}
			>
				Next
			</Button>
		</div>
	</div>
</div>

<CharacterBank bind:show={showCharacterBank} />
