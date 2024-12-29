<script lang="ts">
    import { gameStore } from '../../stores/game';
    import { gameActions } from '../../stores/game-actions';
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card';
    import AnswerProgress from './answer-progress.svelte';

    // Get current game state
    $: alternatives = $gameStore.currentQuestion?.alternatives || [];
    $: questionType = $gameStore.currentQuestion?.type || 'default';
    $: currentPlayer = $gameStore.playerName
        ? $gameStore.players.get($gameStore.playerName)
        : undefined;
    $: hasAnswered = currentPlayer?.hasAnswered || false;
    $: selectedAnswer = currentPlayer?.answer;
    $: timeRemaining = $gameStore.roundDuration;

    // Track which answer was clicked (separate from server state)
    let clickedAnswer: string | null = null;

    // Color mapping with explicit type
    const colorMap: Record<string, string> = {
        'red': '#FF0000',
        'green': '#00FF00',
        'blue': '#0000FF',
        'yellow': '#FFFF00',
        'purple': '#800080',
        'gold': '#FFD700',
        'silver': '#C0C0C0',
        'pink': '#FFC0CB',
        'black': '#000000',
        'white': '#FFFFFF',
        'brown': '#3D251E',
        'orange': '#FFA500',
        'gray': '#808080'
    };

    function handleAnswer(answer: string) {
        if (!hasAnswered) {
            clickedAnswer = answer; // Track local state immediately
            gameActions.submitAnswer(answer);
        }
    }

    function getButtonStyles(alternative: string) {
        const styles = [];
        
        // Base styles
        styles.push('aspect-square', 'rounded-lg', 'transition-all', 'duration-300', 'relative');

        // Add a strong border to the selected answer
        if (alternative === clickedAnswer) {
            styles.push('ring-4', 'ring-primary', 'ring-offset-2', 'scale-105', 'z-10');
        }

        // If any answer is clicked, reduce opacity of non-selected answers
        if (clickedAnswer && alternative !== clickedAnswer) {
            styles.push('opacity-40');
        }

        // Hover state only before answer is selected
        if (!clickedAnswer) {
            styles.push('hover:ring-2', 'hover:ring-muted-foreground', 'hover:scale-105');
        }

        // Question type specific styles
        if (questionType === 'character') {
            styles.push('p-0', 'overflow-hidden');
        } else if (questionType !== 'color') {
            styles.push('bg-muted');
        }

        // Cursor styles
        styles.push(clickedAnswer ? 'cursor-not-allowed' : 'cursor-pointer');

        return styles.join(' ');
    }
</script>

<div class="container mx-auto max-w-2xl space-y-6 p-6">
    <!-- Answer Progress -->
    <Card>
        <CardContent class="p-4">
            <AnswerProgress />
        </CardContent>
    </Card>

    <!-- Answer Options -->
    <Card>
        <CardHeader>
            <CardTitle class="flex items-center justify-between">
                <span>Choose your answer</span>
                <span class="text-muted-foreground">{timeRemaining}s</span>
            </CardTitle>
        </CardHeader>
        <CardContent>
            <div class="grid grid-cols-2 gap-4">
                {#each alternatives as alternative}
                    <button
                        class={getButtonStyles(alternative)}
                        disabled={clickedAnswer !== null}
                        on:click={() => handleAnswer(alternative)}
                        style={questionType === 'color' ? `background-color: ${colorMap[alternative.toLowerCase()]};` : ''}
                    >
                        {#if questionType === 'character'}
                            <img
                                src={`http://192.168.1.155:8765/img_avif/${alternative}.avif`}
                                alt={alternative}
                                class="h-full w-full object-cover"
                            />
                        {:else if questionType === 'color'}
                            <span class="sr-only">{alternative}</span>
                        {:else}
                            <div class="flex h-full w-full items-center justify-center text-lg font-medium">
                                {alternative}
                            </div>
                        {/if}
                    </button>
                {/each}
            </div>
        </CardContent>
    </Card>
</div>
