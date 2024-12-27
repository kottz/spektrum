<script lang="ts">
    import { Button } from '$lib/components/ui/button';
    import { Input } from '$lib/components/ui/input';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import { gameActions } from '../../stores/game-actions';
    import { gameStore } from '../../stores/game';

    export let playerName = "";
    export let lobbyCode = "";
    
    let isJoining = false;
    let isCreating = false;
    let joinError = '';

    const handleCreateLobby = async () => {
        if (isCreating) return;
        
        try {
            isCreating = true;
            await gameActions.createGame();
        } catch (error) {
            console.error("Error creating lobby:", error);
            alert('Failed to create lobby. Please try again.');
        } finally {
            isCreating = false;
        }
    };

    const handleJoinGame = async () => {
        if (!lobbyCode || !playerName) {
            alert('Please enter both lobby code and player name');
            return;
        }

        try {
            isJoining = true;
            joinError = '';
            await gameActions.joinGame(lobbyCode, playerName);
        } catch (error) {
            console.error("Error joining game:", error);
            joinError = 'Failed to join game. Please check your code and try again.';
        } finally {
            isJoining = false;
        }
    };
</script>

<div class="container flex min-h-screen flex-col items-center justify-center gap-8 py-8">
    <div class="flex items-center gap-3">
        <span class="text-2xl">🎵</span>
        <h1 class="text-3xl font-bold">Music Quiz</h1>
    </div>

    <div class="grid w-full max-w-lg gap-6">
        <!-- Create Lobby Card -->
        <Card class="border-zinc-800 bg-zinc-900/50 shadow-xl">
            <CardHeader>
                <CardTitle>Create New Lobby</CardTitle>
            </CardHeader>
            <CardContent>
                <Button
                    size="lg"
                    class="w-full bg-primary font-medium hover:bg-primary/90"
                    on:click={handleCreateLobby}
                    disabled={isCreating}
                >
                    {isCreating ? 'Creating...' : 'Create Lobby'}
                </Button>
            </CardContent>
        </Card>

        <!-- Join Lobby Card -->
        <Card class="border-zinc-800 bg-zinc-900/50 shadow-xl">
            <CardHeader>
                <CardTitle>Join Game</CardTitle>
            </CardHeader>
            <CardContent class="grid gap-4">
                <Input
                    name="lobbyCode"
                    placeholder="Enter lobby code"
                    bind:value={lobbyCode}
                    class="border-zinc-800 bg-zinc-900/50"
                    disabled={isJoining}
                />
                <Input
                    name="playerName"
                    placeholder="Enter your name"
                    bind:value={playerName}
                    class="border-zinc-800 bg-zinc-900/50"
                    disabled={isJoining}
                />
                {#if joinError}
                    <div class="text-sm text-red-400">
                        {joinError}
                    </div>
                {/if}
                <Button
                    size="lg"
                    class="w-full bg-primary font-medium hover:bg-primary/90"
                    on:click={handleJoinGame}
                    disabled={isJoining || !lobbyCode || !playerName}
                >
                    {isJoining ? 'Joining...' : 'Join Game'}
                </Button>
            </CardContent>
        </Card>
    </div>
</div>
