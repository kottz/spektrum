<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { goto } from '$app/navigation';
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-8 px-3 py-8">
	<div class="grid w-full max-w-lg gap-6 text-center">
		<h2 class="text-xl font-semibold">Invalid lobby code</h2>
		<p class="text-muted-foreground">
			The code you entered doesn’t match the valid 6 digit format.
		</p>

		<Button onclick={() => goto('/')} class="mx-auto">Back to home</Button>
	</div>
</div>
