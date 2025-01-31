<script lang="ts">
    import { Card } from '$lib/components/ui/card';
    import { gameStore } from '$lib/stores/game.svelte';
    
    const joinCode = $derived(gameStore.state.joinCode);
</script>

<Card class="bg-card">
    <div class="p-6 flex flex-col items-center justify-center">
        <h2 class="text-xl font-semibold mb-2">Join Code</h2>
        <div class="text-3xl font-mono font-bold">{joinCode}</div>
    </div>
</Card>
