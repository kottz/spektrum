<script lang="ts">
    import * as Alert from "$lib/components/ui/alert";
    import { notifications } from '../stores/notification-store';
    import { fly } from 'svelte/transition';
</script>

<div class="fixed top-4 right-4 z-50 flex flex-col gap-2 max-w-md">
    {#each $notifications as notification (notification.id)}
        <div transition:fly={{ x: 300, duration: 300 }}>
            <Alert.Root variant={notification.type}>
                <Alert.Title>
                    {notification.type === 'destructive' ? 'Error' : 
                     notification.type === 'success' ? 'Success' : 'Notice'}
                </Alert.Title>
                <Alert.Description>
                    {notification.message}
                </Alert.Description>
            </Alert.Root>
        </div>
    {/each}
</div>
