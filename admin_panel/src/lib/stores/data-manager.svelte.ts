import type {
	StoredData,
	Media,
	Character,
	Question,
	QuestionOption,
	QuestionSet
} from '$lib/types';

interface AdminState {
	media: Media[];
	characters: Character[];
	questions: Question[];
	options: QuestionOption[];
	sets: QuestionSet[];
	isLoading: boolean;
	error: string | null;
}

interface AdminStoreState extends AdminState {
	snapshots: AdminState[];
	currentIndex: number;
	maxSnapshots: number;
	isBatching: boolean;
	preBatchState: AdminState | null;
}

const initialState: AdminState = {
	media: [],
	characters: [],
	questions: [],
	options: [],
	sets: [],
	isLoading: false,
	error: null
};

function createAdminStore() {
	const state = $state<AdminStoreState>({
		...initialState,
		snapshots: [],
		currentIndex: -1,
		maxSnapshots: 50,
		isBatching: false,
		preBatchState: null
	});

	function takeSnapshot() {
		if (state.isBatching) return;

		// Truncate forward history if we're not at the end
		if (state.currentIndex < state.snapshots.length - 1) {
			state.snapshots = state.snapshots.slice(0, state.currentIndex + 1);
		}

		// Create new snapshot with raw values
		const snapshot: AdminState = {
			media: [...state.media],
			characters: [...state.characters],
			questions: [...state.questions],
			options: [...state.options],
			sets: [...state.sets],
			isLoading: state.isLoading,
			error: state.error
		};

		state.snapshots = [...state.snapshots, snapshot];

		// Maintain max size
		if (state.snapshots.length > state.maxSnapshots) {
			state.snapshots = state.snapshots.slice(1);
		} else {
			state.currentIndex++;
		}
	}

	return {
		setData: (data: StoredData) => {
			state.media = data.media;
			state.questions = data.questions;
			state.options = data.options;
			state.sets = data.sets;
			takeSnapshot();
		},

		setLoading: (loading: boolean) => {
			state.isLoading = loading;
		},

		setError: (error: string | null) => {
			state.error = error;
		},

		startBatch: () => {
			if (state.isBatching) {
				console.error('Batch already in progress');
				return;
			}
			state.isBatching = true;
			state.preBatchState = {
				media: [...state.media],
				characters: [...state.characters],
				questions: [...state.questions],
				options: [...state.options],
				sets: [...state.sets],
				isLoading: state.isLoading,
				error: state.error
			};
		},

		commitBatch: () => {
			if (!state.isBatching) {
				console.error('No batch to commit');
				return;
			}
			state.isBatching = false;
			takeSnapshot();
			state.preBatchState = null;
		},

		cancelBatch: () => {
			if (!state.isBatching) {
				console.error('No batch to cancel');
				return;
			}
			const pb = state.preBatchState!;
			state.media = [...pb.media];
			state.questions = [...pb.questions];
			state.options = [...pb.options];
			state.sets = [...pb.sets];
			state.isLoading = pb.isLoading;
			state.error = pb.error;
			state.isBatching = false;
			state.preBatchState = null;
		},

		addEntity: (entityType: keyof StoredData, entity: any) => {
			state[entityType] = [...state[entityType], entity];
			takeSnapshot();
		},

		modifyEntity: (
			entityType: keyof StoredData,
			id: number,
			changes:
				| Partial<Media>
				| Partial<Character>
				| Partial<Question>
				| Partial<QuestionOption>
				| Partial<QuestionSet>
		) => {
			switch (entityType) {
				case 'media':
					state.media = state.media.map((item) =>
						item.id === id ? { ...item, ...(changes as Partial<Media>) } : item
					);
					break;
				case 'characters':
					state.characters = state.characters.map((item) =>
						item.id === id ? { ...item, ...(changes as Partial<Character>) } : item
					);
					break;
				case 'questions':
					state.questions = state.questions.map((item) =>
						item.id === id ? { ...item, ...(changes as Partial<Question>) } : item
					);
					break;
				case 'options':
					state.options = state.options.map((item) =>
						item.id === id ? { ...item, ...(changes as Partial<QuestionOption>) } : item
					);
					break;
				case 'sets':
					state.sets = state.sets.map((item) =>
						item.id === id ? { ...item, ...(changes as Partial<QuestionSet>) } : item
					);
					break;
			}
			takeSnapshot();
		},

		// Delete cascade rules:
		// - Delete Media → Delete all its Questions → Delete their Options
		// - Delete Character → Delete all Options using its name
		// - Delete Question → Delete all its Options
		// - Delete Set → Remove from Set-Question relationships
		deleteEntity: (entityType: keyof StoredData, id: number) => {
			switch (entityType) {
				case 'media': {
					// Get all questions for this media
					const mediaQuestions = state.questions.filter((q) => q.media_id === id);
					const questionIds = mediaQuestions.map((q) => q.id);
					// Delete all options for those questions
					state.options = state.options.filter((o) => !questionIds.includes(o.question_id));
					// Delete the questions
					state.questions = state.questions.filter((q) => q.media_id !== id);
					// Remove question references from sets
					state.sets = state.sets.map((s) => ({
						...s,
						question_ids: s.question_ids.filter((qid) => !questionIds.includes(qid))
					}));
					// Delete the media
					state.media = state.media.filter((m) => m.id !== id);
					break;
				}
				case 'characters': {
					// Get the character name before deleting
					const character = state.characters.find((c) => c.id === id);
					if (character) {
						// Delete all options that use this character's name
						state.options = state.options.filter((o) => o.option_text !== character.name);
					}
					// Delete the character
					state.characters = state.characters.filter((c) => c.id !== id);
					break;
				}
				case 'questions': {
					// Delete all options for this question
					state.options = state.options.filter((o) => o.question_id !== id);
					// Remove from any sets
					state.sets = state.sets.map((s) => ({
						...s,
						question_ids: s.question_ids.filter((qid) => qid !== id)
					}));
					// Delete the question
					state.questions = state.questions.filter((q) => q.id !== id);
					break;
				}
				case 'sets': {
					// Just delete the set (no cascading needed)
					state.sets = state.sets.filter((s) => s.id !== id);
					break;
				}
				case 'options': {
					// Simple delete
					state.options = state.options.filter((o) => o.id !== id);
					break;
				}
			}
			takeSnapshot();
		},

		undo: () => {
			if (state.currentIndex > 0) {
				state.currentIndex--;
				const snapshot = state.snapshots[state.currentIndex];
				state.media = snapshot.media;
				state.questions = snapshot.questions;
				state.options = snapshot.options;
				state.sets = snapshot.sets;
			}
		},

		redo: () => {
			if (state.currentIndex < state.snapshots.length - 1) {
				state.currentIndex++;
				const snapshot = state.snapshots[state.currentIndex];
				state.media = snapshot.media;
				state.questions = snapshot.questions;
				state.options = snapshot.options;
				state.sets = snapshot.sets;
			}
		},

		canUndo: () => state.currentIndex > 0,
		canRedo: () => state.currentIndex < state.snapshots.length - 1,

		reset: () => {
			Object.assign(state, initialState);
			state.snapshots = [];
			state.currentIndex = -1;
		},

		getState: () => ({
			media: state.media,
			characters: state.characters,
			questions: state.questions,
			options: state.options,
			sets: state.sets
		}),
		getSnapshots: () => state.snapshots,
		getSnapshotIndex: () => state.currentIndex,
		isLoading: () => state.isLoading,
		getError: () => state.error
	};
}

export const adminStore = createAdminStore();
