<script lang="ts">
	import { PUBLIC_SPEKTRUM_CDN_URL } from '$env/static/public';
	import type { Character, Media } from '$lib/types';
	import type { Question, QuestionOption } from '$lib/types';
	import { QuestionType, Color } from '$lib/types';

	import { adminStore } from '$lib/stores/data-manager.svelte';

	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import * as Command from '$lib/components/ui/command';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import CharacterBank from '$lib/components/character-bank.svelte';

	import TableContainer from './table/table-container.svelte';
	import SearchInput from './table/search-input.svelte';
	import Pagination from './table/pagination.svelte';
	import EditableInput from './table/editable-input.svelte';

	import { Check, ChevronsUpDown } from 'lucide-svelte';
	import { cn } from '$lib/utils';

	const state = $state({
		currentPage: 0,
		itemsPerPage: 10,
		searchTerm: '',
		forceShowCharacterBank: false,
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
		tempOptions: [] as QuestionOption[],
		tempOptionCounter: -1,
		sortKey: 'id' as 'id' | 'media',
		sortDirection: 'asc' as 'asc' | 'desc'
	});

	const showCharacterBank = $derived(
		() =>
			(state.isAddingQuestion && state.newQuestionData.question_type === QuestionType.Character) ||
			state.forceShowCharacterBank
	);

	const mediaById = $derived(new Map(adminStore.getState().media.map((m) => [m.id, m])));

	const optionsByQuestionId = $derived(
		adminStore.getState().options.reduce((map, opt) => {
			map.set(opt.question_id, [...(map.get(opt.question_id) || []), opt]);
			return map;
		}, new Map<number, QuestionOption[]>())
	);

	const filteredData = $derived(() => {
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

	const totalPages = $derived(Math.ceil(filteredData().length / state.itemsPerPage));
	const paginatedData = $derived(() =>
		filteredData().slice(
			state.currentPage * state.itemsPerPage,
			(state.currentPage + 1) * state.itemsPerPage
		)
	);

	const filteredMediaOptions = $derived(() =>
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
				id: Math.max(0, ...adminStore.getState().options.map((o) => o.id)) + 1,
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
		const s = new Set(state.selectedTypes);
		s.has(t) ? s.delete(t) : s.add(t);
		state.selectedTypes = s;
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
		const id = Math.max(0, ...adminStore.getState().questions.map((q) => q.id)) + 1;
		state.newQuestionData = {
			id,
			media_id: 0,
			question_type: '',
			question_text: '',
			image_url: null,
			is_active: true
		};
		state.tempOptions = [];
		state.isAddingQuestion = true;
	}

	function handleSaveQuestion() {
		try {
			adminStore.startBatch();
			adminStore.addEntity('questions', { ...state.newQuestionData });

			let next = Math.max(0, ...adminStore.getState().options.map((o) => o.id)) + 1;
			state.tempOptions.forEach((o) =>
				adminStore.addEntity('options', { ...o, id: next++, question_id: state.newQuestionData.id })
			);

			adminStore.commitBatch();
		} finally {
			adminStore.cancelBatch();
		}

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
		adminStore.cancelBatch();
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
	<div class={showCharacterBank() ? 'h-1/2' : 'h-full'}>
		<TableContainer>
			<!-- header -->
			<svelte:fragment slot="header-left">
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
						on:click={() => (state.forceShowCharacterBank = !state.forceShowCharacterBank)}
					>
						Toggle Character Bank
					</Button>
				</div>
			</svelte:fragment>

			<svelte:fragment slot="header-right">
				<Button on:click={handleAddQuestion}>Add Question</Button>
			</svelte:fragment>

			<!-- table -->
			<ScrollArea class="h-[calc(100%-4rem)]">
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>
								<div class="flex items-center gap-1">
									ID
									<Button variant="ghost" size="sm" on:click={() => handleSort('id')}>
										{state.sortKey === 'id' ? (state.sortDirection === 'asc' ? '↑' : '↓') : '↕'}
									</Button>
								</div>
							</Table.Head>
							<Table.Head class="w-48">
								<div class="flex items-center gap-1">
									Media
									<Button variant="ghost" size="sm" on:click={() => handleSort('media')}>
										{state.sortKey === 'media' ? (state.sortDirection === 'asc' ? '↑' : '↓') : '↕'}
									</Button>
								</div>
							</Table.Head>
							<Table.Head class="pl-1">
								<DropdownMenu.Root>
									<DropdownMenu.Trigger asChild let:builder>
										<Button variant="outline" size="sm" builders={[builder]}>Type ↓</Button>
									</DropdownMenu.Trigger>
									<DropdownMenu.Content class="w-56">
										<DropdownMenu.Label>Question Types</DropdownMenu.Label>
										<DropdownMenu.Separator />
										{#each Object.values(QuestionType) as t}
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
								<Table.Cell>{state.newQuestionData.id}</Table.Cell>

								<!-- media picker -->
								<Table.Cell>
									<Popover.Root>
										<Popover.Trigger asChild let:builder>
											<Button
												builders={[builder]}
												variant="outline"
												role="combobox"
												class="w-full justify-between"
											>
												{state.newQuestionData.media_id
													? getMediaInfo(state.newQuestionData.media_id)?.title
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
												{#each filteredMediaOptions() as m}
													<div
														class="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 hover:bg-gray-100 dark:hover:bg-gray-800"
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
													</div>
												{/each}
												{#if filteredMediaOptions().length === 0}
													<div class="px-2 py-1.5 text-gray-500">No media found</div>
												{/if}
											</div>
										</Popover.Content>
									</Popover.Root>
								</Table.Cell>

								<!-- type picker -->
								<Table.Cell>
									<Popover.Root>
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
													{#each Object.values(QuestionType) as t}
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
									{#if state.newQuestionData.question_type === QuestionType.Color}
										<!-- unchanged color logic -->
										<Popover.Root>
											<Popover.Trigger asChild let:builder>
												<Button
													builders={[builder]}
													variant="outline"
													class="w-full justify-between"
												>
													{#if state.tempOptions.length > 0}
														{state.tempOptions.length}
														{#each state.tempOptions as o}{' - ' + o.option_text}{/each}
													{:else}
														Select colors...
													{/if}
													<ChevronsUpDown class="ml-2 h-4 w-4 shrink-0 opacity-50" />
												</Button>
											</Popover.Trigger>
											<Popover.Content class="w-[200px] p-0">
												<Command.Root>
													<Command.Group>
														{#each Object.values(Color) as c}
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
									{:else if state.newQuestionData.question_type === QuestionType.Character}
										<!-- character drag-drop -->
										<div
											class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
											ondragover={(e) => e.preventDefault()}
											ondrop={(e) => handleDrop(e, state.newQuestionData.id)}
										>
											{#each state.tempOptions as o (o.id)}
												{@const char = adminStore
													.getState()
													.characters.find((c: Character) => c.name === o.option_text)}
												<div class="group relative">
													<div
														class="flex cursor-pointer flex-col items-center"
														onclick={() => {
															o.is_correct = !o.is_correct;
															state.tempOptions = [...state.tempOptions];
														}}
													>
														<img
															src={char?._pendingImage?.dataUrl ||
																(char?.image_url && PUBLIC_SPEKTRUM_CDN_URL
																	? `${PUBLIC_SPEKTRUM_CDN_URL}/${char.image_url}`
																	: char?.image_url) ||
																`/img/${o.option_text}.avif`}
															alt={o.option_text}
															class="h-12 w-12 rounded transition-transform hover:scale-105"
															class:ring-2={o.is_correct}
															class:ring-green-500={o.is_correct}
														/>
														<span class="mt-1 w-12 truncate text-center text-xs">
															{o.option_text}
														</span>
													</div>
													<button
														class="absolute -right-2 -top-2 hidden h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white group-hover:flex"
														onclick={() =>
															(state.tempOptions = state.tempOptions.filter((x) => x.id !== o.id))}
													>
														×
													</button>
												</div>
											{/each}
										</div>
									{:else if state.newQuestionData.question_type === QuestionType.Text}
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
														on:click={() => {
															o.is_correct = !o.is_correct;
															state.tempOptions = [...state.tempOptions];
														}}>✓</Button
													>
													<Button
														size="icon"
														variant="outline"
														class="text-red-600"
														on:click={() =>
															(state.tempOptions = state.tempOptions.filter((x) => x.id !== o.id))}
														>×</Button
													>
												</div>
											{/each}
											<Button
												variant="ghost"
												size="sm"
												on:click={() =>
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
									{/if}
								</Table.Cell>

								<Table.Cell><Switch bind:checked={state.newQuestionData.is_active} /></Table.Cell>
								<Table.Cell class="text-right">
									<div class="flex justify-end gap-2">
										<Button size="sm" variant="outline" on:click={handleSaveQuestion}>Save</Button>
										<Button
											size="sm"
											variant="outline"
											class="text-red-600"
											on:click={handleCancelAdd}>Cancel</Button
										>
									</div>
								</Table.Cell>
							</Table.Row>
						{/if}

						<!-- existing rows -->
						{#each paginatedData() as q (q.id)}
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
												src={PUBLIC_SPEKTRUM_CDN_URL
													? `${PUBLIC_SPEKTRUM_CDN_URL}/${q.image_url}`
													: q.image_url}
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
									{#if q.question_type === QuestionType.Character}
										<!-- character viewer -->
										<div
											class="flex min-h-[60px] flex-wrap gap-2 rounded-lg border-2 border-dashed border-gray-300 p-2"
											ondragover={(e) => e.preventDefault()}
											ondrop={(e) => handleDrop(e, q.id)}
										>
											{#each getQuestionOptions(q.id) as opt}
												{@const char = adminStore
													.getState()
													.characters.find((c: Character) => c.name === opt.option_text)}
												<div class="group relative">
													<div
														class="flex cursor-pointer flex-col items-center"
														onclick={() => toggleCorrectOption(opt)}
													>
														<img
															src={char?._pendingImage?.dataUrl ||
																(char?.image_url && PUBLIC_SPEKTRUM_CDN_URL
																	? `${PUBLIC_SPEKTRUM_CDN_URL}/${char.image_url}`
																	: char?.image_url) ||
																`/img/${opt.option_text}.avif`}
															alt={opt.option_text}
															class="h-12 w-12 rounded transition-transform hover:scale-105"
															class:ring-2={opt.is_correct}
															class:ring-green-500={opt.is_correct}
														/>
														<span
															class="mt-1 w-12 truncate text-center text-xs"
															class:text-green-600={opt.is_correct}>{opt.option_text}</span
														>
													</div>
													<button
														class="absolute -right-2 -top-2 hidden h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white group-hover:flex"
														onclick={(e) => {
															e.stopPropagation();
															removeOption(q.id, opt.id);
														}}>×</button
													>
												</div>
											{/each}
										</div>
									{:else if q.question_type === QuestionType.Text}
										<!-- text options w/EditableInput -->
										<div class="flex flex-col gap-2">
											{#each getQuestionOptions(q.id) as opt}
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
														on:click={() => toggleCorrectOption(opt)}>✓</Button
													>
													<Button
														size="icon"
														variant="outline"
														class="text-red-600"
														on:click={() => removeOption(q.id, opt.id)}>×</Button
													>
												</div>
											{/each}
											<Button
												variant="ghost"
												size="sm"
												on:click={() =>
													adminStore.addEntity('options', {
														id: Math.max(0, ...adminStore.getState().options.map((o) => o.id)) + 1,
														question_id: q.id,
														option_text: '',
														is_correct: false
													})}>Add option</Button
											>
										</div>
									{:else}
										<div>
											{#each getQuestionOptions(q.id) as opt, i}
												<span class:text-green-600={opt.is_correct}>
													{opt.option_text}{i < getQuestionOptions(q.id).length - 1 ? ', ' : ''}
												</span>
											{/each}
										</div>
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
										on:click={() => handleDeleteQuestion(q.id)}>Delete</Button
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
				totalItems={filteredData().length}
				itemsPerPage={state.itemsPerPage}
				onPageChange={(p) => (state.currentPage = p)}
			/>
		</TableContainer>
	</div>

	{#if showCharacterBank()}
		<div class="h-1/2">
			<CharacterBank />
		</div>
	{/if}
</div>
