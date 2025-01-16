<script lang="ts">
    import * as Table from '$lib/components/ui/table';
    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';
    import { adminStore } from '$lib/stores/admin-data';
    import type { Question } from '$lib/types';
    import { QuestionType } from '$lib/types';

    // Pagination state
    let currentPage = 0;
    let itemsPerPage = 10;
    let searchTerm = '';

    // Filtered and paginated data
    $: filteredData = $adminStore.questions.filter(
        (question) =>
            question.question_text?.toLowerCase().includes(searchTerm.toLowerCase()) ||
            question.id.toString().includes(searchTerm)
    );

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
        </div>
        <Button on:click={handleAddQuestion}>Add Question</Button>
    </div>

    <div class="rounded-md border">
        <Table.Root>
            <Table.Header>
                <Table.Row>
                    <Table.Head>ID</Table.Head>
                    <Table.Head>Media</Table.Head>
                    <Table.Head>Type</Table.Head>
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
                            {question.question_text || 'No question text'}
                        </Table.Cell>
                        <Table.Cell>
                            <div class="flex flex-col gap-1">
                                {#each getQuestionOptions(question.id) as option}
                                    <div class:text-green-600={option.is_correct}>
                                        {option.option_text}
                                    </div>
                                {/each}
                            </div>
                        </Table.Cell>
                        <Table.Cell>
                            <span class={`inline-flex rounded-full px-2 py-1 text-xs font-semibold
                                ${question.is_active ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}`}>
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
