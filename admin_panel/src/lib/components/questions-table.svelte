<script lang="ts">
	import { characterImageSrc, mediaUrl } from '#lib/media.ts';
	import { COLORS, QUESTION_TYPES, type Character } from '#lib/types.ts';
	import type { Color } from '#lib/generated/Color.ts';
	import type { Media } from '#lib/generated/Media.ts';
	import type { Question } from '#lib/generated/Question.ts';
	import type { QuestionOption } from '#lib/generated/QuestionOption.ts';
	import type { QuestionType } from '#lib/generated/QuestionType.ts';

	import { adminStore } from '#lib/stores/data-manager.svelte.ts';
	import { questionProblem } from '#lib/question-rules.ts';

	import * as Table from '#lib/components/ui/table/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.ts';
	import { Input } from '#lib/components/ui/input/index.ts';
	import { Switch } from '#lib/components/ui/switch/index.ts';
	import * as Popover from '#lib/components/ui/popover/index.ts';
	import * as Command from '#lib/components/ui/command/index.ts';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.ts';
	import CharacterBank from '#lib/components/character-bank.svelte';

	import TableContainer from './table/table-container.svelte';
	import SearchInput from './table/search-input.svelte';
	import Pagination from './table/pagination.svelte';
	import EditableInput from './table/editable-input.svelte';

	import { Check, ChevronsUpDown } from '@lucide/svelte';
	import { cn } from '#lib/utils.ts';
	import { SvelteSet } from 'svelte/reactivity';

	/**
	 * A question being composed. It has no real id until saved (DRAFT_ID only
	 * routes drops to the draft row); its type stays empty until one is picked.
	 */
	type QuestionDraft = Omit<Question, 'question_type'> & { question_type: QuestionType | '' };
	const DRAFT_ID = 0;

	function emptyQuestionDraft(): QuestionDraft {
		return {
			id: DRAFT_ID,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		};
	}

	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: '',
		forceShowCharacterBank: false,
		selectedTypes: new SvelteSet(QUESTION_TYPES),
		mediaSearchTerm: '',
		isAddingQuestion: false,
		newQuestionData: emptyQuestionDraft(),
		tempOptions: [] as QuestionOption[],
		tempOptionCounter: -1,
		sortKey: 'id' as 'id' | 'media',
		sortDirection: 'asc' as 'asc' | 'desc'
	});

	const showCharacterBank = $derived(
		(state.isAddingQuestion && state.newQuestionData.question_type === 'character') ||
			state.forceShowCharacterBank
	);

	const mediaById = $derived(new Map(adminStore.getState().media.map((m) => [m.id, m])));

	const optionsByQuestionId = $derived(
		adminStore.getState().options.reduce((map, opt) => {
			map.set(opt.question_id, [...(map.get(opt.question_id) || []), opt]);
			return map;
		}, new Map<number, QuestionOption[]>())
	);

	const filteredData = $derived.by(() => {
		const s = state.searchTerm.toLowerCase();
		return adminStore
			.getState()
			.questions.filter((q) => {
				if (!state.selectedTypes.has(q.question_type)) return false;
				const m = mediaById.get(q.media_id);
				const opts = optionsByQuestionId.get(q.id) || [];
				return (
					q.id.toString().includes(state.searchTerm) ||
					q.question_text?.toLowerCase().includes(s) ||
					m?.title.toLowerCase().includes(s) ||
					opts.some((o) => o.option_text.toLowerCase().includes(s))
				);
			})
			.slice()
			.sort((a, b) => {
				if (state.sortKey === 'id')
					return state.sortDirection === 'asc' ? a.id - b.id : b.id - a.id;
				const aTitle = mediaById.get(a.media_id)?.title?.toLowerCase() || '';
				const bTitle = mediaById.get(b.media_id)?.title?.toLowerCase() || '';
				const cmp = aTitle.localeCompare(bTitle);
				return state.sortDirection === 'asc' ? cmp : -cmp;
			});
	});

	const totalPages = $derived(Math.ceil(filteredData.length / state.itemsPerPage));
	const paginatedData = $derived.by(() =>
		filteredData.slice(
			state.currentPage * state.itemsPerPage,
			(state.currentPage + 1) * state.itemsPerPage
		)
	);

	const filteredMediaOptions = $derived.by(() =>
		adminStore
			.getState()
			.media.filter((m) => {
				const s = state.mediaSearchTerm.toLowerCase();
				if (!s) return true;
				return (
					m.title?.toLowerCase().includes(s) ||
					m.artist?.toLowerCase().includes(s) ||
					m.id.toString().includes(s)
				);
			})
			.slice(0, 5)
	);

	function getMediaInfo(id: number): Media | null {
		return adminStore.getState().media.find((m) => m.id === id) || null;
	}
	function getQuestionOptions(id: number) {
		return adminStore.getState().options.filter((o) => o.question_id === id);
	}

	function handleSort(key: 'id' | 'media') {
		if (state.sortKey === key) {
			state.sortDirection = state.sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			state.sortKey = key;
			state.sortDirection = 'asc';
		}
		state.currentPage = 0;
	}

	function handleDrop(e: DragEvent, qId: number) {
		e.preventDefault();
		const name = e.dataTransfer?.getData('text/plain');
		if (!name) return;

		if (state.isAddingQuestion && qId === state.newQuestionData.id) {
			state.tempOptions = [
				...state.tempOptions,
				{ id: state.tempOptionCounter--, question_id: qId, option_text: name, is_correct: false }
			];
		} else {
			adminStore.addEntity('options', {
				question_id: qId,
				option_text: name,
				is_correct: false
			});
		}
	}

	function toggleCorrectOption(o: QuestionOption) {
		adminStore.modifyEntity('options', o.id, { ...o, is_correct: !o.is_correct });
	}
	function removeOption(_: number, id: number) {
		adminStore.deleteEntity('options', id);
	}

	function toggleQuestionType(t: QuestionType) {
		if (state.selectedTypes.has(t)) {
			state.selectedTypes.delete(t);
		} else {
			state.selectedTypes.add(t);
		}
	}

	function toggleColorOption(c: Color) {
		state.tempOptions = state.tempOptions.some((o) => o.option_text === c)
			? state.tempOptions.filter((o) => o.option_text !== c)
			: [
					...state.tempOptions,
					{
						id: state.tempOptionCounter--,
						question_id: state.newQuestionData.id,
						option_text: c,
						is_correct: true
					}
				];
	}

	function handleAddQuestion() {
		state.newQuestionData = emptyQuestionDraft();
		state.tempOptions = [];
		state.isAddingQuestion = true;
	}

	function handleSaveQuestion() {
		const { question_type } = state.newQuestionData;
		if (!question_type || !state.newQuestionData.media_id) {
			alert('Select media and a question type before saving.');
			return;
		}
		const problem = questionProblem(
			question_type,
			state.newQuestionData.question_text,
			state.tempOptions
		);
		if (problem) {
			alert(problem);
			return;
		}

		adminStore.startBatch();
		try {
			const { id: _draftId, ...draft } = state.newQuestionData;
			const questionId = adminStore.addEntity('questions', { ...draft, question_type });
			for (const { id: _tempId, ...option } of state.tempOptions) {
				adminStore.addEntity('options', { ...option, question_id: questionId });
			}

			adminStore.commitBatch();
		} catch (error) {
			adminStore.cancelBatch();
			throw error;
		}

		resetQuestionDraft();
	}

	/** A year question has one option: the correct year. */
	function setDraftYear(value: string) {
		const year = value.trim();
		state.tempOptions = year
			? [{ id: -1, question_id: state.newQuestionData.id, option_text: year, is_correct: true }]
			: [];
	}

	function setYear(questionId: number, option: QuestionOption | undefined, value: string) {
		const year = value.trim();
		if (option) {
			adminStore.modifyEntity('options', option.id, { option_text: year, is_correct: true });
		} else if (year) {
			adminStore.addEntity('options', {
				question_id: questionId,
				option_text: year,
				is_correct: true
			});
		}
	}

	function handleCancelAdd() {
		resetQuestionDraft();
	}

	function resetQuestionDraft() {
		state.isAddingQuestion = false;
		state.newQuestionData = emptyQuestionDraft();
		state.tempOptions = [];
		state.tempOptionCounter = -1;
	}

	function handleDeleteQuestion(id: number) {
		adminStore.deleteEntity('questions', id);
	}

	function toggleActiveQuestion(q: Question) {
		adminStore.modifyEntity('questions', q.id, { ...q, is_active: !q.is_active });
	}
</script>

<div class="flex h-full flex-col">
	<div class={showCharacterBank ? 'h-1/2' : 'h-full'}>
		<TableContainer>
			<!-- header -->
			{#snippet headerLeft()}
				<div class="flex items-center gap-4">
					<SearchInput
						value={state.searchTerm}
						placeholder="Search questions..."
						onInput={(v) => {
							state.searchTerm = v;
							state.currentPage = 0;
						}}
					/>
					<Button
						variant="outline"
						onclick={() => (state.forceShowCharacterBank = !state.forceShowCharacterBank)}
					>
						Toggle Character Bank
					</Button>
				</div>
			{/snippet}

			{#snippet headerRight()}
				<Button onclick={handleAddQuestion}>Add Question</Button>
			{/snippet}

			<!-- table -->
			<ScrollArea class="h-[calc(100%-4rem)]">
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>
								<div class="flex items-center gap-1">
									ID
									<Button variant="ghost" size="sm" onclick={() => handleSort('id')}>
										{state.sortKey === 'id' ? (state.sortDirection === 'asc' ? '↑' : '↓') : '↕'}
									</Button>
								</div>
							</Table.Head>
							<Table.Head class="w-48">
								<div class="flex items-center gap-1">
									Media
									<Button variant="ghost" size="sm" onclick={() => handleSort('media')}>
										{state.sortKey === 'media' ? (state.sortDirection === 'asc' ? '↑' : '↓') : '↕'}
									</Button>
								</div>
							</Table.Head>
							<Table.Head class="pl-1">
								<DropdownMenu.Root>
									<DropdownMenu.Trigger>
										{#snippet child({ props })}
											<Button variant="outline" size="sm" {...props}>Type ↓</Button>
										{/snippet}
									</DropdownMenu.Trigger>
									<DropdownMenu.Content class="w-56">
										<DropdownMenu.Label>Question Types</DropdownMenu.Label>
										<DropdownMenu.Separator />
										{#each QUESTION_TYPES as t (t)}
											<DropdownMenu.CheckboxItem
												checked={state.selectedTypes.has(t)}
												onCheckedChange={() => toggleQuestionType(t)}
											>
												{t.charAt(0).toUpperCase() + t.slice(1)}
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
						<!-- new row -->
						{#if state.isAddingQuestion}
							<Table.Row class="bg-blue-50 dark:bg-gray-800">
								<Table.Cell class="text-muted-foreground">New</Table.Cell>

								<!-- media picker -->
								<Table.Cell>
									<Popover.Root>
										<Popover.Trigger>
											{#snippet child({ props })}
												<Button
													{...props}
													variant="outline"
													role="combobox"
													class="w-full justify-between"
												>
													{state.newQuestionData.media_id
														? getMediaInfo(state.newQuestionData.media_id)?.title
														: 'Select media...'}
													<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
												</Button>
											{/snippet}
										</Popover.Trigger>
										<Popover.Content class="w-[300px] p-2">
											<Input
												type="text"
												placeholder="Search media..."
												bind:value={state.mediaSearchTerm}
												class="mb-2"
											/>
											<div class="max-h-[200px] overflow-y-auto">
												{#each filteredMediaOptions as m (m.id)}
													<button
														type="button"
														class="flex w-full cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-left hover:bg-gray-100 dark:hover:bg-gray-800"
														onclick={() => {
															state.newQuestionData.media_id = m.id;
															state.mediaSearchTerm = '';
														}}
													>
														<Check
															class={cn(
																'h-4 w-4',
																state.newQuestionData.media_id === m.id
																	? 'text-blue-500'
																	: 'text-transparent'
															)}
														/>
														<span>{m.title} - {m.artist}</span>
													</button>
												{/each}
												{#if filteredMediaOptions.length === 0}
													<div class="px-2 py-1.5 text-gray-500">No media found</div>
												{/if}
											</div>
										</Popover.Content>
									</Popover.Root>
								</Table.Cell>

								<!-- type picker -->
								<Table.Cell>
									<Popover.Root>
										<Popover.Trigger>
											{#snippet child({ props })}
												<Button
													{...props}
													variant="outline"
													role="combobox"
													class="w-full justify-between"
												>
													{state.newQuestionData.question_type || 'Select type...'}
													<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
												</Button>
											{/snippet}
										</Popover.Trigger>
										<Popover.Content class="w-[200px] p-0">
											<Command.Root>
												<Command.List>
													<Command.Group>
														{#each QUESTION_TYPES as t (t)}
															<Command.Item
																value={t}
																onSelect={() => {
																	state.newQuestionData.question_type = t;
																	state.tempOptions = [];
																}}
															>
																<Check
																	class={cn(
																		'mr-2 h-4 w-4',
																		state.newQuestionData.question_type !== t && 'text-transparent'
																	)}
																/>
																{t}
															</Command.Item>
														{/each}
													</Command.Group>
												</Command.List>
											</Command.Root>
										</Popover.Content>
									</Popover.Root>
								</Table.Cell>

								<!-- new question text -->
								<Table.Cell>
									<Input
										type="text"
										placeholder="Question text..."
										bind:value={state.newQuestionData.question_text}
									/>
								</Table.Cell>

								<!-- temp options -->
								<Table.Cell class="w-[400px] min-w-[200px]">
									{#if state.newQuestionData.question_type === 'color'}
										<!-- unchanged color logic -->
										<Popover.Root>
											<Popover.Trigger>
												{#snippet child({ props })}
													<Button {...props} variant="outline" class="w-full justify-between">
														{#if state.tempOptions.length > 0}
															{state.tempOptions.length}
															{#each state.tempOptions as o (o.id)}{' - ' + o.option_text}{/each}
														{:else}
															Select colors...
														{/if}
														<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
													</Button>
												{/snippet}
											</Popover.Trigger>
											<Popover.Content class="w-[200px] p-0">
												<Command.Root>
													<Command.Group>
														{#each COLORS as c (c)}
															<Command.Item value={c} onSelect={() => toggleColorOption(c)}>
																<Check
																	class={cn(
																		'mr-2 h-4 w-4',
																		state.tempOptions.some((o) => o.option_text === c)
																			? 'opacity-100'
																			: 'opacity-0'
																	)}
																/>
																{c}
															</Command.Item>
														{/each}
													</Command.Group>
												</Command.Root>
											</Popover.Content>
										</Popover.Root>
									{:else if state.newQuestionData.question_type === 'character'}
										<!-- character drag-drop -->
										<div
											role="region"
											aria-label="Character options, drop characters here"
											class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
											ondragover={(e) => e.preventDefault()}
											ondrop={(e) => handleDrop(e, state.newQuestionData.id)}
										>
											{#each state.tempOptions as o (o.id)}
												{@const char = adminStore
													.getState()
													.characters.find((c: Character) => c.name === o.option_text)}
												<div class="group relative">
													<button
														type="button"
														aria-pressed={o.is_correct}
														class="flex cursor-pointer flex-col items-center"
														onclick={() => {
															o.is_correct = !o.is_correct;
															state.tempOptions = [...state.tempOptions];
														}}
													>
														<img
															src={char
																? characterImageSrc(char)
																: mediaUrl(`img/${o.option_text}.avif`)}
															alt={o.option_text}
															class="h-12 w-12 rounded transition-transform hover:scale-105"
															class:ring-2={o.is_correct}
															class:ring-green-500={o.is_correct}
														/>
														<span class="mt-1 w-12 truncate text-center text-xs">
															{o.option_text}
														</span>
													</button>
													<button
														type="button"
														aria-label="Remove option"
														class="absolute -top-2 -right-2 hidden h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white group-hover:flex"
														onclick={() =>
															(state.tempOptions = state.tempOptions.filter((x) => x.id !== o.id))}
													>
														×
													</button>
												</div>
											{/each}
										</div>
									{:else if state.newQuestionData.question_type === 'text'}
										<!-- use EditableInput instead of textarea -->
										<div class="flex flex-col gap-2">
											{#each state.tempOptions as o (o.id)}
												<div class="flex items-start gap-2">
													<div
														class="flex-1 rounded border-2 p-1"
														class:border-green-500={o.is_correct}
														class:border-gray-300={!o.is_correct}
													>
														<EditableInput
															value={o.option_text}
															placeholder="Option…"
															onChange={(v) => {
																o.option_text = v;
																state.tempOptions = [...state.tempOptions];
															}}
															onCommit={() => {
																state.tempOptions = [...state.tempOptions];
															}}
														/>
													</div>
													<Button
														size="icon"
														variant="outline"
														onclick={() => {
															o.is_correct = !o.is_correct;
															state.tempOptions = [...state.tempOptions];
														}}>✓</Button
													>
													<Button
														size="icon"
														variant="outline"
														class="text-red-600"
														onclick={() =>
															(state.tempOptions = state.tempOptions.filter((x) => x.id !== o.id))}
														>×</Button
													>
												</div>
											{/each}
											<Button
												variant="ghost"
												size="sm"
												onclick={() =>
													(state.tempOptions = [
														...state.tempOptions,
														{
															id: state.tempOptionCounter--,
															question_id: state.newQuestionData.id,
															option_text: '',
															is_correct: false
														}
													])}
											>
												Add option
											</Button>
										</div>
									{:else if state.newQuestionData.question_type === 'year'}
										<Input
											type="text"
											inputmode="numeric"
											placeholder="Correct year, e.g. 1997"
											value={state.tempOptions[0]?.option_text ?? ''}
											oninput={(e) => setDraftYear(e.currentTarget.value)}
										/>
									{/if}
								</Table.Cell>

								<Table.Cell><Switch bind:checked={state.newQuestionData.is_active} /></Table.Cell>
								<Table.Cell class="text-right">
									<div class="flex justify-end gap-2">
										<Button size="sm" variant="outline" onclick={handleSaveQuestion}>Save</Button>
										<Button
											size="sm"
											variant="outline"
											class="text-red-600"
											onclick={handleCancelAdd}>Cancel</Button
										>
									</div>
								</Table.Cell>
							</Table.Row>
						{/if}

						<!-- existing rows -->
						{#each paginatedData as q (q.id)}
							<Table.Row class="hover:bg-gray-50 dark:hover:bg-gray-800">
								<Table.Cell>{q.id}</Table.Cell>
								<Table.Cell>
									{#if getMediaInfo(q.media_id)?.youtube_id}
										<a
											class="text-blue-600 hover:underline"
											href={`https://youtube.com/watch?v=${getMediaInfo(q.media_id)?.youtube_id}`}
											target="_blank"
											rel="noopener noreferrer"
										>
											{getMediaInfo(q.media_id)?.title || 'Unknown Media'}
										</a>
									{:else}
										{getMediaInfo(q.media_id)?.title || 'Unknown Media'}
									{/if}
								</Table.Cell>
								<Table.Cell>{q.question_type}</Table.Cell>

								<!-- editable question text -->
								<Table.Cell>
									{#if q.image_url}
										<div class="mb-2">
											<img
												src={mediaUrl(q.image_url)}
												alt="Question"
												class="h-12 w-12 rounded object-cover"
											/>
										</div>
									{/if}
									<EditableInput
										value={q.question_text || ''}
										placeholder="Question…"
										onCommit={(v) =>
											adminStore.modifyEntity('questions', q.id, { ...q, question_text: v })}
										onChange={() => {}}
									/>
								</Table.Cell>

								<!-- option column -->
								<Table.Cell class="w-[400px] min-w-[200px]">
									{#if q.question_type === 'character'}
										<!-- character viewer -->
										<div
											role="region"
											aria-label="Character options, drop characters here"
											class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
											ondragover={(e) => e.preventDefault()}
											ondrop={(e) => handleDrop(e, q.id)}
										>
											{#each getQuestionOptions(q.id) as opt (opt.id)}
												{@const char = adminStore
													.getState()
													.characters.find((c: Character) => c.name === opt.option_text)}
												<div class="group relative">
													<button
														type="button"
														aria-pressed={opt.is_correct}
														class="flex cursor-pointer flex-col items-center"
														onclick={() => toggleCorrectOption(opt)}
													>
														<img
															src={char
																? characterImageSrc(char)
																: mediaUrl(`img/${opt.option_text}.avif`)}
															alt={opt.option_text}
															class="h-12 w-12 rounded transition-transform hover:scale-105"
															class:ring-2={opt.is_correct}
															class:ring-green-500={opt.is_correct}
														/>
														<span
															class="mt-1 w-12 truncate text-center text-xs"
															class:text-green-600={opt.is_correct}>{opt.option_text}</span
														>
													</button>
													<button
														type="button"
														aria-label="Remove option"
														class="absolute -top-2 -right-2 hidden h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white group-hover:flex"
														onclick={(e) => {
															e.stopPropagation();
															removeOption(q.id, opt.id);
														}}>×</button
													>
												</div>
											{/each}
										</div>
									{:else if q.question_type === 'text'}
										<!-- text options w/EditableInput -->
										<div class="flex flex-col gap-2">
											{#each getQuestionOptions(q.id) as opt (opt.id)}
												<div class="flex items-start gap-2">
													<div
														class="flex-1 rounded border-2 p-1"
														class:border-green-500={opt.is_correct}
														class:border-gray-300={!opt.is_correct}
													>
														<EditableInput
															value={opt.option_text}
															onCommit={(v) =>
																adminStore.modifyEntity('options', opt.id, {
																	...opt,
																	option_text: v
																})}
															onChange={() => {}}
														/>
													</div>
													<Button
														size="icon"
														variant="outline"
														onclick={() => toggleCorrectOption(opt)}>✓</Button
													>
													<Button
														size="icon"
														variant="outline"
														class="text-red-600"
														onclick={() => removeOption(q.id, opt.id)}>×</Button
													>
												</div>
											{/each}
											<Button
												variant="ghost"
												size="sm"
												onclick={() =>
													adminStore.addEntity('options', {
														question_id: q.id,
														option_text: '',
														is_correct: false
													})}>Add option</Button
											>
										</div>
									{:else if q.question_type === 'year'}
										{@const yearOption = getQuestionOptions(q.id)[0]}
										<EditableInput
											value={yearOption?.option_text ?? ''}
											inputmode="numeric"
											placeholder="Correct year"
											onCommit={(v) => setYear(q.id, yearOption, v)}
											onChange={() => {}}
										/>
									{:else}
										<div>
											{#each getQuestionOptions(q.id) as opt, i (opt.id)}
												<span class:text-green-600={opt.is_correct}>
													{opt.option_text}{i < getQuestionOptions(q.id).length - 1 ? ', ' : ''}
												</span>
											{/each}
										</div>
									{/if}
									{@const problem = questionProblem(
										q.question_type,
										q.question_text,
										getQuestionOptions(q.id)
									)}
									{#if problem}
										<p class="mt-1 text-xs text-red-600">{problem} It is skipped in games.</p>
									{/if}
								</Table.Cell>

								<Table.Cell>
									<button
										class={`inline-flex w-16 justify-center rounded-full border px-2 py-1 text-xs font-semibold ${
											q.is_active
												? 'border-green-300 bg-green-100 text-green-800 dark:bg-green-200'
												: 'border-red-300 bg-red-100 text-red-800'
										} hover:opacity-90`}
										onclick={() => toggleActiveQuestion(q)}
									>
										{q.is_active ? 'Active' : 'Inactive'}
									</button>
								</Table.Cell>

								<Table.Cell class="text-right">
									<Button
										variant="outline"
										size="sm"
										class="text-red-600"
										onclick={() => handleDeleteQuestion(q.id)}>Delete</Button
									>
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</ScrollArea>

			<Pagination
				currentPage={state.currentPage}
				{totalPages}
				totalItems={filteredData.length}
				itemsPerPage={state.itemsPerPage}
				onPageChange={(p) => (state.currentPage = p)}
			/>
		</TableContainer>
	</div>

	{#if showCharacterBank}
		<div class="h-1/2">
			<CharacterBank />
		</div>
	{/if}
</div>
