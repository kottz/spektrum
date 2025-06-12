<script lang="ts">
	import { GamePhase } from '$lib/types/game';
	import { streamStore } from '$lib/stores/stream.store.svelte';
	import StreamQuestionView from './stream-question-view.svelte';
	import StreamScoreboard from './stream-scoreboard.svelte';
	import StreamAnswerProgress from './stream-answer-progress.svelte';

	const gameState = $derived(streamStore.state.gameState);
	const phase = $derived(gameState?.phase);
	const showScoreboard = $derived(phase === GamePhase.Score || phase === GamePhase.GameOver);
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

		const answerContainer = event.currentTarget
			.closest('.answer-resize-container')
			.querySelector('.min-h-0.flex.flex-1');
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
	<div class="flex h-screen flex-col">
		<!-- Header section: Join Code -->
		<header class="mb-4 flex-none p-3">
			<div class="flex items-center justify-center rounded-lg bg-card p-4 shadow">
				<div class="flex items-center gap-2">
					<span class="text-lg text-muted-foreground">Join Code:</span>
					<span class="font-mono text-4xl font-bold">{joinCode}</span>
				</div>
			</div>
		</header>

		<!-- Main Content Area with Resizable Layout -->
		<div class="flex min-h-0 flex-1">
			<!-- Left Panel (Empty space for camera) -->
			<div class="flex-none" style="width: {leftPanelWidth}%">
				<!-- Empty space for streamer's camera -->
			</div>

			<!-- Draggable Divider -->
			<div
				class="flex w-2 cursor-col-resize items-center justify-center bg-border/50 transition-colors hover:bg-border {isDragging
					? 'bg-primary'
					: ''}"
				on:mousedown={startDrag}
				role="separator"
				tabindex="0"
			>
				<div class="h-12 w-0.5 bg-muted-foreground/30"></div>
			</div>

			<!-- Content Area -->
			<div class="flex min-h-0 flex-1 flex-col p-3">
				<div class="grid min-h-0 flex-1 grid-cols-12 gap-4">
					<!-- Left Column: Question Options + Answer Progress -->
					<div class="answer-resize-container col-span-12 flex flex-col space-y-4 lg:col-span-6">
						<!-- Question Options (top) -->
						<div class="flex-none">
							{#if phase === GamePhase.Question}
								<StreamQuestionView />
							{:else if phase === GamePhase.Lobby}
								<div class="flex h-48 items-center justify-center rounded-lg bg-card p-4 shadow">
									<p class="text-xl text-muted-foreground">Waiting for game to start...</p>
								</div>
							{:else}
								<div class="flex h-48 items-center justify-center rounded-lg bg-card p-4 shadow">
									<p class="text-xl text-muted-foreground">Round Over</p>
								</div>
							{/if}
						</div>

						<!-- Answer Progress (bottom) with horizontal resize -->
						<div class="flex min-h-0 flex-1">
							<!-- Empty space (left) -->
							<div class="flex-none" style="width: {100 - answerProgressWidth}%">
								<!-- Empty space for additional camera positioning -->
							</div>

							<!-- Vertical Draggable Divider -->
							<div
								class="flex w-2 cursor-col-resize items-center justify-center bg-border/50 transition-colors hover:bg-border {isDraggingAnswers
									? 'bg-primary'
									: ''}"
								on:mousedown={startAnswerDrag}
								role="separator"
								tabindex="0"
							>
								<div class="h-12 w-0.5 bg-muted-foreground/30"></div>
							</div>

							<!-- Answer Progress (right) -->
							<div class="min-w-0 flex-1" style="width: {answerProgressWidth}%">
								<StreamAnswerProgress />
							</div>
						</div>
					</div>

					<!-- Right Column: Always-visible Scoreboard -->
					<div class="col-span-12 lg:col-span-6">
						<StreamScoreboard />
					</div>
				</div>
			</div>
		</div>
	</div>
{:else}
	<div class="flex h-full items-center justify-center">
		<p>Loading Game View...</p>
	</div>
{/if}
