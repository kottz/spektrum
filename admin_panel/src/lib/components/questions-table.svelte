<script lang="ts">
    import * as Table from '$lib/components/ui/table';
    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';
    import { adminStore } from '$lib/stores/admin-data';
    import type { Question, QuestionOption } from '$lib/types';
    import { QuestionType } from '$lib/types';
    import CharacterBank from '$lib/components/character-bank.svelte';
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";

    // Pagination state
    let currentPage = 0;
    let itemsPerPage = 10;
    let searchTerm = '';
    let showCharacterBank = false;
    let selectedTypes = new Set(Object.values(QuestionType));

    // Filtered and paginated data
    $: filteredData = $adminStore.questions.filter((question) => {
        const searchLower = searchTerm.toLowerCase();
        
        // Check if question type is selected
        if (!selectedTypes.has(question.question_type)) {
            return false;
        }

        // Check question ID and text
        if (question.id.toString().includes(searchLower) ||
            question.question_text?.toLowerCase().includes(searchLower)) {
            return true;
        }
        
        // Check related media title
        const media = $adminStore.media.find(m => m.id === question.media_id);
        if (media?.title.toLowerCase().includes(searchLower)) {
            return true;
        }
        
        // Check character names in options
        const questionOptions = $adminStore.options.filter(opt => opt.question_id === question.id);
        return questionOptions.some(opt => 
            opt.option_text.toLowerCase().includes(searchLower)
        );
    });

    $: totalPages = Math.ceil(filteredData.length / itemsPerPage);

    $: paginatedData = filteredData.slice(
        currentPage * itemsPerPage,
        (currentPage + 1) * itemsPerPage
    );

    // Get media title by ID
    function getMediaTitle(mediaId: number): string {
        const media = $adminStore.media.find(m => m.id === mediaId);
        return media?.title || 'Unknown Media';
    }

    // Get question options
    function getQuestionOptions(questionId: number) {
        return $adminStore.options.filter(opt => opt.question_id === questionId);
    }

    function handleDrop(event: DragEvent, questionId: number) {
        event.preventDefault();
        const charName = event.dataTransfer?.getData('text/plain');
        if (!charName) return;

        // Create a new option for the question
        const newOption: QuestionOption = {
            id: Math.max(0, ...($adminStore.options.map(o => o.id))) + 1, // Temporary ID
            question_id: questionId,
            option_text: charName,
            is_correct: false
        };

        adminStore.update(state => ({
            ...state,
            options: [...state.options, newOption]
        }));
    }

    function removeOption(questionId: number, optionId: number) {
        adminStore.update(state => ({
            ...state,
            options: state.options.filter(opt => opt.id !== optionId)
        }));
    }

    function toggleCorrectOption(option: QuestionOption) {
        adminStore.update(state => ({
            ...state,
            options: state.options.map(opt => 
                opt.id === option.id 
                    ? { ...opt, is_correct: !opt.is_correct }
                    : opt
            )
        }));
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

    function handleAddQuestion() {
        // TODO: Implement add question functionality
        console.log('Add question clicked');
    }

    function handleEditQuestion(question: Question) {
        // TODO: Implement edit question functionality
        console.log('Edit question:', question);
    }

    function handleDeleteQuestion(questionId: number) {
        // TODO: Implement delete question functionality
        console.log('Delete question:', questionId);
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
            {#if $adminStore.questions.some(q => q.question_type.toLowerCase() === QuestionType.Character)}
                <Button variant="outline" on:click={() => showCharacterBank = !showCharacterBank}>
                    Toggle Character Bank
                </Button>
            {/if}
        </div>
        <Button on:click={handleAddQuestion}>Add Question</Button>
    </div>

    <div class="rounded-md border">
        <Table.Root>
            <Table.Header>
                <Table.Row>
                    <Table.Head>ID</Table.Head>
                    <Table.Head>Media</Table.Head>
                    <Table.Head>
                        <div class="flex items-center gap-2">
                            Type
                            <DropdownMenu.Root>
                                <DropdownMenu.Trigger asChild let:builder>
                                    <Button variant="outline" size="sm" builders={[builder]}>
                                        Filter
                                    </Button>
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
                        </div>
                    </Table.Head>
                    <Table.Head>Question</Table.Head>
                    <Table.Head>Options</Table.Head>
                    <Table.Head>Status</Table.Head>
                    <Table.Head class="text-right">Actions</Table.Head>
                </Table.Row>
            </Table.Header>
            <Table.Body>
                {#each paginatedData as question}
                    <Table.Row>
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
                            {#if question.question_type.toLowerCase() === QuestionType.Character}
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
                                                <span class="mt-1 text-center text-xs" class:text-green-600={option.is_correct}>
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
                            <span class={`inline-flex rounded-full px-2 py-1 text-xs font-semibold ${
                                question.is_active ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
                            }`}>
                                {question.is_active ? 'Active' : 'Inactive'}
                            </span>
                        </Table.Cell>
                        <Table.Cell class="text-right">
                            <div class="flex justify-end gap-2">
                                <Button 
                                    variant="outline" 
                                    size="sm"
                                    on:click={() => handleEditQuestion(question)}
                                >
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
            Showing {currentPage * itemsPerPage + 1} to {Math.min(
                (currentPage + 1) * itemsPerPage,
                filteredData.length
            )} of {filteredData.length} questions
        </div>
        <div class="flex gap-2">
            <Button 
                variant="outline" 
                size="sm" 
                on:click={previousPage} 
                disabled={currentPage === 0}
            >
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
