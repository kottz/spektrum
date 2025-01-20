<script lang="ts">
    import { PUBLIC_DEV_ADMIN_PASSWORD } from '$env/static/public';
    import { adminStore } from '$lib/stores/admin-data';
    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';
    import { AlertCircle } from 'lucide-svelte';

    let isSubmitting = false;
    let error: string | null = null;
    let password = '';

    function getEntityTitle(entityType: string, id: number): string {
        switch (entityType) {
            case 'questions':
                return $adminStore.questions.find(q => q.id === id)?.question_text || `Question ${id}`;
            case 'media':
                return $adminStore.media.find(m => m.id === id)?.title || `Media ${id}`;
            case 'options':
                return $adminStore.options.find(o => o.id === id)?.option_text || `Option ${id}`;
            case 'sets':
                return $adminStore.sets.find(s => s.id === id)?.name || `Set ${id}`;
            default:
                return `Unknown ${id}`;
        }
    }

    async function handleApply() {
        if (!password) {
            error = 'Password is required';
            return;
        }

        try {
            isSubmitting = true;
            error = null;

            const response = await fetch('http://localhost:8765/api/update-questions', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    password: password,
                    stored_data: {
                        media: $adminStore.media,
                        questions: $adminStore.questions,
                        options: $adminStore.options,
                        sets: $adminStore.sets
                    }
                })
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            // Reset changes after successful update
            adminStore.setData(await response.json());

        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to apply changes';
        } finally {
            isSubmitting = false;
        }
    }

    function handleCancel() {
        // Reset to original state
        if ($adminStore.originalState) {
            adminStore.setData($adminStore.originalState);
        }
    }
</script>

<div class="space-y-6 rounded-lg border p-6">
    <h2 class="text-xl font-semibold">Pending Changes</h2>

    {#if $adminStore.pendingChanges.length === 0}
        <div class="text-gray-500">No pending changes</div>
    {:else}
        <!-- Added Items -->
        {#if $adminStore.pendingChanges.some(change => change.type === 'added')}
            <div class="space-y-2">
                <h3 class="font-medium text-green-600">Added</h3>
                {#each $adminStore.pendingChanges.filter(c => c.type === 'added') as change}
                    <div class="flex items-center gap-2 rounded bg-green-50 px-3 py-2">
                        <span class="font-medium">{change.entityType}:</span>
                        <span>{getEntityTitle(change.entityType, change.id)}</span>
                    </div>
                {/each}
            </div>
        {/if}

        <!-- Modified Items -->
        {#if $adminStore.pendingChanges.some(change => change.type === 'modified')}
            <div class="space-y-2">
                <h3 class="font-medium text-yellow-600">Modified</h3>
                {#each $adminStore.pendingChanges.filter(c => c.type === 'modified') as change}
                    <div class="flex items-center gap-2 rounded bg-yellow-50 px-3 py-2">
                        <span class="font-medium">{change.entityType}:</span>
                        <span>{getEntityTitle(change.entityType, change.id)}</span>
                    </div>
                {/each}
            </div>
        {/if}

        <!-- Deleted Items -->
        {#if $adminStore.pendingChanges.some(change => change.type === 'deleted')}
            <div class="space-y-2">
                <h3 class="font-medium text-red-600">Deleted</h3>
                {#each $adminStore.pendingChanges.filter(c => c.type === 'deleted') as change}
                    <div class="flex items-center gap-2 rounded bg-red-50 px-3 py-2">
                        <span class="font-medium">{change.entityType}:</span>
                        <span>{getEntityTitle(change.entityType, change.id)}</span>
                    </div>
                {/each}
            </div>
        {/if}

        <!-- Error Message -->
        {#if error}
            <div class="flex items-center gap-2 rounded-md bg-red-50 p-3 text-red-600">
                <AlertCircle class="h-5 w-5" />
                <span>{error}</span>
            </div>
        {/if}

        <!-- Actions -->
        <div class="space-y-4 pt-4">
            <div>
                <Input
                    type="password"
                    placeholder="Enter password to apply changes"
                    bind:value={password}
                />
            </div>
            <div class="flex justify-end gap-3">
                <Button
                    variant="outline"
                    on:click={handleCancel}
                    disabled={isSubmitting}
                >
                    Cancel
                </Button>
                <Button
                    variant="default"
                    on:click={handleApply}
                    disabled={isSubmitting || !password}
                >
                    {isSubmitting ? 'Applying...' : 'Apply Changes'}
                </Button>
            </div>
        </div>
    {/if}
</div>
