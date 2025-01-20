// admin-data.ts
import { writable } from 'svelte/store';
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

	function trackChange(
		type: ChangeType,
		entityType: 'media' | 'questions' | 'options' | 'sets',
		id: number,
		oldValue?: any,
		newValue?: any
	) {
		update((state) => {
			const changes = [...state.pendingChanges];
			const existingChangeIndex = changes.findIndex(
				(c) => c.entityType === entityType && c.id === id
			);

			if (existingChangeIndex !== -1) {
				changes[existingChangeIndex] = { type, entityType, id, oldValue, newValue };
			} else {
				changes.push({ type, entityType, id, oldValue, newValue });
			}

			return { ...state, pendingChanges: changes };
		});
	}

	return {
		subscribe,
		update: (updater: (state: AdminStore) => AdminStore) => update(updater),
		setData: (data: StoredData) => {
			update((state) => ({
				...state,
				media: data.media,
				questions: data.questions,
				options: data.options,
				sets: data.sets,
				originalState: JSON.parse(JSON.stringify(data)), // Deep copy of original state
				pendingChanges: []
			}));
		},
		setLoading: (loading: boolean) => {
			update((state) => ({ ...state, isLoading: loading }));
		},
		setError: (error: string | null) => {
			update((state) => ({ ...state, error }));
		},
		markForDeletion: (entityType: 'media' | 'questions' | 'options' | 'sets', id: number) => {
			update((state) => {
				const original = state.originalState?.[entityType].find((item) => item.id === id);
				if (original) {
					trackChange('deleted', entityType, id, original, undefined);
				}
				return state;
			});
		},
		undoDelete: (entityType: 'media' | 'questions' | 'options' | 'sets', id: number) => {
			update((state) => ({
				...state,
				pendingChanges: state.pendingChanges.filter(
					(change) => !(change.entityType === entityType && change.id === id)
				)
			}));
		},
		addEntity: (entityType: 'media' | 'questions' | 'options' | 'sets', entity: any) => {
			update((state) => {
				trackChange('added', entityType, entity.id, undefined, entity);
				return {
					...state,
					[entityType]: [...state[entityType], entity]
				};
			});
		},
		modifyEntity: (
			entityType: 'media' | 'questions' | 'options' | 'sets',
			id: number,
			changes: Partial<any>
		) => {
			update((state) => {
				const original = state.originalState?.[entityType].find((item) => item.id === id);
				const current = state[entityType].find((item) => item.id === id);
				if (current) {
					const updated = { ...current, ...changes };
					trackChange('modified', entityType, id, original, updated);
					return {
						...state,
						[entityType]: state[entityType].map((item) => (item.id === id ? updated : item))
					};
				}
				return state;
			});
		},
		reset: () => set(initialState),
		getPendingChanges: () => {
			let state: AdminStore;
			update((s) => {
				state = s;
				return s;
			});
			return state.pendingChanges;
		}
	};
}

export const adminStore = createAdminStore();
