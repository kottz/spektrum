// admin-data.ts
import { writable, get } from 'svelte/store';
import type { StoredData, Media, Question, QuestionOption, QuestionSet } from '$lib/types';

export const activeTab = writable<'media' | 'questions' | 'sets'>('media');
// Types for tracking changes
type ChangeType = 'added' | 'deleted' | 'modified';
interface Change {
	type: ChangeType;
	entityType: 'media' | 'questions' | 'options' | 'sets';
	id: number;
	oldValue?: any;
	newValue?: any;
}

interface AdminStore {
	media: Media[];
	questions: Question[];
	options: QuestionOption[];
	sets: QuestionSet[];
	isLoading: boolean;
	error: string | null;
	originalState?: StoredData; // Keep track of initial state
	pendingChanges: Change[];
}

const initialState: AdminStore = {
	media: [],
	questions: [],
	options: [],
	sets: [],
	isLoading: false,
	error: null,
	originalState: undefined,
	pendingChanges: []
};

function createAdminStore() {
	const { subscribe, set, update } = writable<AdminStore>(initialState);

	let currentChanges: Change[] = [];

	function addChange(change: Change) {
		console.log('Adding change:', change);
		currentChanges = [...currentChanges, change];
		update(state => ({
			...state,
			pendingChanges: [...currentChanges]
		}));
		console.log('Current changes after update:', currentChanges);
	}

	function updatePendingChanges() {
		update(state => ({
			...state,
			pendingChanges: [...currentChanges]
		}));
	}

	return {
		subscribe,
		update: (updater: (state: AdminStore) => AdminStore) => update(updater),
		setData: (data: StoredData) => {
			currentChanges = []; // Reset changes when new data is set
			update((state) => ({
				...state,
				media: data.media,
				questions: data.questions,
				options: data.options,
				sets: data.sets,
				originalState: JSON.parse(JSON.stringify(data)),
				pendingChanges: []
			}));
		},
		setLoading: (loading: boolean) => {
			update((state) => ({ ...state, isLoading: loading }));
		},
		setError: (error: string | null) => {
			update((state) => ({ ...state, error }));
		},
		addEntity: (entityType: 'media' | 'questions' | 'options' | 'sets', entity: any) => {
			addChange({
				type: 'added',
				entityType,
				id: entity.id,
				newValue: entity
			});

			update(state => ({
				...state,
				[entityType]: [...state[entityType], entity],
				pendingChanges: [...currentChanges]
			}));
		},
		markForDeletion: (entityType: 'media' | 'questions' | 'options' | 'sets', id: number) => {
			update(state => {
				const original = state.originalState?.[entityType].find(item => item.id === id);
				if (original) {
					addChange({
						type: 'deleted',
						entityType,
						id,
						oldValue: original
					});
				}
				return {
					...state,
					pendingChanges: [...currentChanges]
				};
			});
		},
		undoDelete: (entityType: 'media' | 'questions' | 'options' | 'sets', id: number) => {
			currentChanges = currentChanges.filter(
				change => !(change.entityType === entityType && change.id === id)
			);
			updatePendingChanges();
		},
		modifyEntity: (
			entityType: 'media' | 'questions' | 'options' | 'sets',
			id: number,
			changes: Partial<any>
		) => {
			update(state => {
				const original = state.originalState?.[entityType].find(item => item.id === id);
				const current = state[entityType].find(item => item.id === id);
				if (current) {
					const updated = { ...current, ...changes };
					addChange({
						type: 'modified',
						entityType,
						id,
						oldValue: original,
						newValue: updated
					});
					return {
						...state,
						[entityType]: state[entityType].map(item =>
							item.id === id ? updated : item
						)
					};
				}
				return state;
			});
		},
		getOptionIdsForQuestion: (questionId: number) => {
			const state = get(adminStore);
			return state.options
				.filter(option => option.question_id === questionId)
				.map(option => option.id);
		},
		reset: () => {
			currentChanges = [];
			set(initialState);
		},
		getPendingChanges: () => currentChanges,
		getFinalState: () => {
			const state = get(adminStore);
			const currentState = {
				media: [...state.media],
				questions: [...state.questions],
				options: [...state.options],
				sets: [...state.sets]
			};

			console.log('Current state:', currentState);

			// Apply pending changes to get final state
			for (const change of state.pendingChanges) {
				if (change.type === 'deleted') {
					currentState[change.entityType] = currentState[change.entityType].filter(
						item => item.id !== change.id
					);
				} else if (change.type === 'added') {
					continue; // Already in current state
				} else if (change.type === 'modified') {
					const index = currentState[change.entityType].findIndex(item => item.id === change.id);
					if (index !== -1) {
						currentState[change.entityType][index] = change.newValue;
					}
				}
			}

			return {
				...currentState
			};
		}
	};
}

export const adminStore = createAdminStore();
