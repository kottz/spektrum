<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { uiStore } from '$lib/stores/ui.store.svelte';
	import { broadcastService } from '$lib/services/broadcast.service';
	import { MonitorPlay, Eye, EyeOff, ExternalLink } from 'lucide-svelte';
	import { info } from '$lib/utils/logger';

	const streamWindowState = $derived(uiStore.streamWindow);
	let isStreamContentVisible = $state(true);

	function handleOpenStreamWindow(): void {
		info('StreamControls: Opening stream window');
		uiStore.openStreamWindow();

		// Initialize broadcast service for admin if not already done
		if (!broadcastService.getIsInitialized() && !broadcastService.getIsStreamWindow()) {
			broadcastService.initialize(false); // false = admin window
		}
	}

	function handleCloseStreamWindow(): void {
		info('StreamControls: Closing stream window');
		uiStore.closeStreamWindow();
	}

	function handleToggleVisibility(): void {
		if (isStreamContentVisible) {
			info('StreamControls: Hiding stream content');
			broadcastService.broadcastStreamControl('hide');
		} else {
			info('StreamControls: Showing stream content');
			broadcastService.broadcastStreamControl('show');
		}
		isStreamContentVisible = !isStreamContentVisible;
	}

	function handleFocusStream(): void {
		if (streamWindowState.window && !streamWindowState.window.closed) {
			streamWindowState.window.focus();
		}
	}
</script>

<Card>
	<CardHeader>
		<CardTitle class="flex items-center gap-2">
			<MonitorPlay class="h-5 w-5" />
			Stream View
		</CardTitle>
	</CardHeader>
	<CardContent class="space-y-2">
		{#if streamWindowState.isOpen}
			<Button onclick={handleFocusStream} variant="outline" size="sm" class="w-full">
				<MonitorPlay class="mr-2 h-4 w-4" /> Focus Window
			</Button>
			<Button onclick={handleToggleVisibility} variant="outline" size="sm" class="w-full">
				{#if isStreamContentVisible}
					<Eye class="mr-2 h-4 w-4" /> Hide Content
				{:else}
					<EyeOff class="mr-2 h-4 w-4" /> Show Content
				{/if}
			</Button>
			<Button onclick={handleCloseStreamWindow} variant="destructive" size="sm" class="w-full">
				<ExternalLink class="mr-2 h-4 w-4" /> Close Window
			</Button>
		{:else}
			<Button onclick={handleOpenStreamWindow} variant="default" size="sm" class="w-full">
				<MonitorPlay class="mr-2 h-4 w-4" /> Open Stream Window
			</Button>
		{/if}
	</CardContent>
</Card>
