<script lang="ts">
	import * as Alert from '$lib/components/ui/alert';
	import { notifications } from '../stores/notification-store';
	import { fly } from 'svelte/transition';
</script>

<div class="fixed right-4 top-4 z-50 flex max-w-md flex-col gap-2">
	{#each $notifications as notification (notification.id)}
		<div transition:fly={{ x: 300, duration: 300 }} class="bg-background">
			<Alert.Root variant={notification.type}>
				<Alert.Title>
					{notification.type === 'destructive'
						? 'Error'
						: notification.type === 'success'
							? 'Success'
							: 'Notice'}
				</Alert.Title>
				<Alert.Description>
					{notification.message}
				</Alert.Description>
			</Alert.Root>
		</div>
	{/each}
</div>
