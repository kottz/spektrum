<script lang="ts">
	import StartButton from './controls/start-game-button.svelte';
	import EndLeaveButton from './controls/end-leave-button.svelte';
	import RoundButton from './controls/round-control-button.svelte';
	import SkipButton from './controls/skip-control-button.svelte';
</script>

<div class="border-t bg-background">
	<div class="space-y-3 p-4">
		<!-- Top Row -->
		<div class="grid grid-cols-2 gap-3">
			<StartButton />
			<EndLeaveButton />
		</div>
		<!-- Bottom Row -->
		<div class="grid grid-cols-2 gap-3">
			<RoundButton />
			<SkipButton />
		</div>
	</div>
</div>
