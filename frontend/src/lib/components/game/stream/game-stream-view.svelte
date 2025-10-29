<script lang="ts">
	import { GamePhase } from '$lib/types/game';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import StreamQuestionView from './stream-question-view.svelte';
	import StreamScoreboard from './stream-scoreboard.svelte';
	import StreamAnswerProgress from './stream-answer-progress.svelte';
	import StreamLobbyView from './stream-lobby-view.svelte';
	import StreamTimer from './stream-timer.svelte';
	import StreamGameOver from './stream-game-over.svelte';

	const gameState = $derived(streamStore.state.gameState);
	const phase = $derived(gameState?.phase);
	const joinCode = $derived(gameState?.joinCode || 'N/A');

	let leftPanelWidth = $state(0); // Default: 0% (full width for content)
	let isDragging = $state(false);
	let answerProgressWidth = $state(100); // Default: 100% (full width for answer progress)
	let isDraggingAnswers = $state(false);

	function startDrag(event: MouseEvent) {
		isDragging = true;
		event.preventDefault();

		const handleMouseMove = (e: MouseEvent) => {
			if (!isDragging) return;
			const containerWidth = window.innerWidth;
			const newWidth = (e.clientX / containerWidth) * 100;
			leftPanelWidth = Math.max(0, Math.min(50, newWidth)); // Limit between 0% and 50%
		};

		const handleMouseUp = () => {
			isDragging = false;
			document.removeEventListener('mousemove', handleMouseMove);
			document.removeEventListener('mouseup', handleMouseUp);
		};

		document.addEventListener('mousemove', handleMouseMove);
		document.addEventListener('mouseup', handleMouseUp);
	}

	function startAnswerDrag(event: MouseEvent) {
		isDraggingAnswers = true;
		event.preventDefault();

		const target = event.currentTarget as HTMLElement;
		const containerElement = target.closest('.answer-resize-container');
		if (!containerElement) return;

		const answerContainer = containerElement.querySelector('.min-h-0.flex.flex-1') as HTMLElement;
		if (!answerContainer) return;

		const handleMouseMove = (e: MouseEvent) => {
			if (!isDraggingAnswers) return;
			const rect = answerContainer.getBoundingClientRect();
			const relativeX = e.clientX - rect.left;
			const newLeftSpaceWidth = (relativeX / rect.width) * 100;
			answerProgressWidth = Math.max(30, Math.min(100, 100 - newLeftSpaceWidth)); // Invert the calculation
		};

		const handleMouseUp = () => {
			isDraggingAnswers = false;
			document.removeEventListener('mousemove', handleMouseMove);
			document.removeEventListener('mouseup', handleMouseUp);
		};

		document.addEventListener('mousemove', handleMouseMove);
		document.addEventListener('mouseup', handleMouseUp);
	}
</script>

{#if gameState}
	<div class="flex h-dvh flex-col">
		<!-- Header section: App Name, Join Code and Timer -->
		<header class="flex-none p-3">
			<div class="bg-card flex items-center justify-between rounded-lg p-4 shadow-sm">
				<div class="flex flex-1">
					<span class="text-4xl font-bold">Melodiquiz.se</span>
				</div>
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-lg">Join Code:</span>
					<span class="font-mono text-4xl font-bold">{joinCode}</span>
				</div>
				<div class="flex flex-1 justify-end">
					<StreamTimer />
				</div>
			</div>
		</header>

		<!-- Main Content Area -->
		{#if phase === GamePhase.Lobby}
			<!-- Full-screen lobby view -->
			<div class="flex min-h-0 flex-1">
				<StreamLobbyView />
			</div>
		{:else if phase === GamePhase.GameOver}
			<!-- Full-screen game over view -->
			<div class="flex min-h-0 flex-1">
				<StreamGameOver />
			</div>
		{:else}
			<!-- Resizable Layout for Game Phases -->
			<div class="flex min-h-0 flex-1">
				<!-- Left Panel (Empty space for camera) -->
				<div class="flex-none" style="width: {leftPanelWidth}%">
					<!-- Empty space for streamer's camera -->
				</div>

				<!-- Draggable Divider -->
				<button
					class="bg-border/50 hover:bg-border flex w-2 cursor-col-resize items-center justify-center transition-colors {isDragging
						? 'bg-primary'
						: ''}"
					onmousedown={startDrag}
					aria-label="Resize left panel"
				>
					<div class="bg-muted-foreground/30 h-12 w-0.5"></div>
				</button>

				<!-- Content Area -->
				<div class="flex min-h-0 flex-1 flex-col">
					<div class="grid min-h-0 flex-1 grid-cols-12 gap-4 overflow-hidden">
						<!-- Left Column: Question Options + Timer + Answer Progress -->
						<div
							class="answer-resize-container col-span-12 flex min-h-0 flex-1 flex-col space-y-4 lg:col-span-6"
						>
							<!-- Question Options (top) -->
							<div class="flex-none">
								{#if phase === GamePhase.Question}
									<StreamQuestionView />
								{:else}
									<div
										class="bg-card flex h-96 items-center justify-center rounded-lg p-4 shadow-sm"
									></div>
								{/if}
							</div>

							<!-- Answer Progress (bottom) with horizontal resize -->
							<div class="flex min-h-0 flex-1">
								<!-- Empty space (left) -->
								<div class="flex-none" style="width: {100 - answerProgressWidth}%">
									<!-- Empty space for additional camera positioning -->
								</div>

								<!-- Vertical Draggable Divider -->
								<button
									class="bg-border/50 hover:bg-border flex w-2 cursor-col-resize items-center justify-center transition-colors {isDraggingAnswers
										? 'bg-primary'
										: ''}"
									onmousedown={startAnswerDrag}
									aria-label="Resize answer progress panel"
								>
									<div class="bg-muted-foreground/30 h-12 w-0.5"></div>
								</button>

								<!-- Answer Progress (right) -->
								<div class="min-w-0 flex-1" style="width: {answerProgressWidth}%">
									<StreamAnswerProgress />
								</div>
							</div>
						</div>

						<!-- Right Column: Always-visible Scoreboard -->
						<div class="col-span-12 flex min-h-0 flex-1 overflow-hidden lg:col-span-6">
							<StreamScoreboard />
						</div>
					</div>
				</div>
			</div>
		{/if}
	</div>
{:else}
	<div class="flex h-full items-center justify-center">
		<p>Loading Game View...</p>
	</div>
{/if}
