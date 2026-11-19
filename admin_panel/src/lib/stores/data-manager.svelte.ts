import type { Character } from '#lib/types.ts';
import type { Media } from '#lib/generated/Media.ts';
import type { Question } from '#lib/generated/Question.ts';
import type { QuestionOption } from '#lib/generated/QuestionOption.ts';
import type { QuestionSet } from '#lib/generated/QuestionSet.ts';
import type { DataVersion } from '#lib/generated/DataVersion.ts';
import type { StoredData } from '#lib/generated/StoredData.ts';
import type { StoredDataResponse } from '#lib/generated/StoredDataResponse.ts';

/** The editor's working copy of the stored data. */
interface AdminState {
	media: Media[];
	characters: Character[];
	questions: Question[];
	options: QuestionOption[];
	sets: QuestionSet[];
}

interface HistoryItem {
	state: AdminState;
	message: string;
}

interface AdminStoreState extends AdminState {
	/**
	 * The server version these edits start from; a save is refused if the server
	 * has moved on. Null until data is loaded, i.e. until the admin logs in.
	 */
	baseVersion: DataVersion | null;
	snapshots: HistoryItem[];
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
	sets: []
};

function createAdminStore() {
	const state = $state<AdminStoreState>({
		...initialState,
		baseVersion: null,
		snapshots: [],
		currentIndex: -1,
		maxSnapshots: 100,
		isBatching: false,
		preBatchState: null
	});

	function takeSnapshot(message: string) {
		if (state.isBatching) return;

		// Truncate forward history if we're not at the end
		if (state.currentIndex < state.snapshots.length - 1) {
			state.snapshots = state.snapshots.slice(0, state.currentIndex + 1);
		}

		// Create new snapshot with raw values
		const snapshot: HistoryItem = {
			state: {
				media: [...state.media],
				// Entities are replaced, never mutated, and pending File objects are
				// immutable, so snapshots can share them
				characters: [...state.characters],
				questions: [...state.questions],
				options: [...state.options],
				sets: [...state.sets]
			},
			message
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
		/** Loads server data as a fresh starting point, discarding any previous history. */
		setData: ({ stored_data, version }: StoredDataResponse) => {
			Object.assign(state, {
				...stored_data,
				baseVersion: version,
				snapshots: [],
				currentIndex: -1,
				isBatching: false,
				preBatchState: null
			});
			takeSnapshot('Initial state');
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
				sets: [...state.sets]
			};
		},

		commitBatch: () => {
			if (!state.isBatching) {
				console.error('No batch to commit');
				return;
			}
			state.isBatching = false;
			takeSnapshot('Committed batch operation');
			state.preBatchState = null;
		},

		cancelBatch: () => {
			if (!state.isBatching) {
				console.error('No batch to cancel');
				return;
			}
			const pb = state.preBatchState!;
			state.media = [...pb.media];
			state.characters = [...pb.characters];
			state.questions = [...pb.questions];
			state.options = [...pb.options];
			state.sets = [...pb.sets];
			state.isBatching = false;
			state.preBatchState = null;
		},

		/**
		 * Adds an entity and returns its id. Ids are allocated here, at insertion,
		 * so a form opened earlier can never claim an id that undo/redo has since
		 * handed back to another entity.
		 */
		addEntity: <K extends keyof AdminState>(
			entityType: K,
			entity: Omit<AdminState[K][number], 'id'>
		): number => {
			const id = Math.max(0, ...state[entityType].map((e) => e.id)) + 1;
			state[entityType] = [...state[entityType], { ...entity, id }] as AdminStoreState[K];
			takeSnapshot(`Added ${entityType} ${id}`);
			return id;
		},

		modifyEntity: (
			entityType: keyof AdminState,
			id: number,
			changes: Partial<Media | Character | Question | QuestionOption | QuestionSet>
		) => {
			// Create a descriptive message based on the changes
			let message = `Modified ${entityType} ${id}`;
			if ('title' in changes) {
				message = `Changed ${entityType} title to "${changes.title}" (ID: ${id})`;
			} else if ('name' in changes) {
				message = `Changed ${entityType} name to "${changes.name}" (ID: ${id})`;
			} else if ('question_text' in changes) {
				message = `Modified question text (ID: ${id})`;
			}

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
			takeSnapshot(message);
		},

		deleteEntity: (entityType: keyof AdminState, id: number) => {
			const message = `Deleted ${entityType} ${id}`;
			let additionalInfo = '';

			switch (entityType) {
				case 'media': {
					const mediaTitle = state.media.find((m) => m.id === id)?.title;
					const mediaQuestions = state.questions.filter((q) => q.media_id === id);
					const questionIds = mediaQuestions.map((q) => q.id);
					const optionCount = state.options.filter((o) =>
						questionIds.includes(o.question_id)
					).length;

					additionalInfo = ` (${mediaTitle}) with ${mediaQuestions.length} questions and ${optionCount} options`;

					state.options = state.options.filter((o) => !questionIds.includes(o.question_id));
					state.questions = state.questions.filter((q) => q.media_id !== id);
					state.sets = state.sets.map((s) => ({
						...s,
						question_ids: s.question_ids.filter((qid) => !questionIds.includes(qid))
					}));
					state.media = state.media.filter((m) => m.id !== id);
					break;
				}
				case 'characters': {
					const character = state.characters.find((c) => c.id === id);
					if (character) {
						const optionCount = state.options.filter(
							(o) => o.option_text === character.name
						).length;
						additionalInfo = ` (${character.name}) affecting ${optionCount} options`;
						state.options = state.options.filter((o) => o.option_text !== character.name);
					}
					state.characters = state.characters.filter((c) => c.id !== id);
					break;
				}
				case 'questions': {
					const questionText = state.questions.find((q) => q.id === id)?.question_text;
					const optionCount = state.options.filter((o) => o.question_id === id).length;
					additionalInfo = questionText
						? ` ("${questionText}") with ${optionCount} options`
						: ` with ${optionCount} options`;

					state.options = state.options.filter((o) => o.question_id !== id);
					state.sets = state.sets.map((s) => ({
						...s,
						question_ids: s.question_ids.filter((qid) => qid !== id)
					}));
					state.questions = state.questions.filter((q) => q.id !== id);
					break;
				}
				case 'sets': {
					const setName = state.sets.find((s) => s.id === id)?.name;
					additionalInfo = setName ? ` (${setName})` : '';
					state.sets = state.sets.filter((s) => s.id !== id);
					break;
				}
				case 'options': {
					const option = state.options.find((o) => o.id === id);
					additionalInfo = option ? ` ("${option.option_text}")` : '';
					state.options = state.options.filter((o) => o.id !== id);
					break;
				}
			}
			takeSnapshot(message + additionalInfo);
		},

		undo: () => {
			if (state.currentIndex > 0) {
				state.currentIndex--;
				const snapshot = state.snapshots[state.currentIndex].state;
				state.media = snapshot.media;
				state.characters = snapshot.characters;
				state.questions = snapshot.questions;
				state.options = snapshot.options;
				state.sets = snapshot.sets;
			}
		},

		redo: () => {
			if (state.currentIndex < state.snapshots.length - 1) {
				state.currentIndex++;
				const snapshot = state.snapshots[state.currentIndex].state;
				state.media = snapshot.media;
				state.characters = snapshot.characters;
				state.questions = snapshot.questions;
				state.options = snapshot.options;
				state.sets = snapshot.sets;
			}
		},

		canUndo: () => state.currentIndex > 0,
		canRedo: () => state.currentIndex < state.snapshots.length - 1,

		getState: () => ({
			media: state.media,
			characters: state.characters,
			questions: state.questions,
			options: state.options,
			sets: state.sets
		}),
		getSnapshots: () => state.snapshots,
		getSnapshotIndex: () => state.currentIndex,
		isLoaded: () => state.baseVersion !== null,
		getBaseVersion: () => state.baseVersion,

		/** The data as the server stores it, without client-only pending uploads. */
		exportData: (): StoredData => ({
			media: state.media,
			characters: state.characters.map(({ _pendingImage, ...character }) => character),
			questions: state.questions,
			options: state.options,
			sets: state.sets
		})
	};
}

export const adminStore = createAdminStore();
