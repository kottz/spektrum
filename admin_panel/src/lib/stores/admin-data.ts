import { writable } from 'svelte/store';
import type { StoredData, Media, Question, QuestionOption, QuestionSet } from '$lib/types';

export const activeTab = writable<'media' | 'questions' | 'sets'>('media');

interface AdminStore {
	media: Media[];
	questions: Question[];
	options: QuestionOption[];
	sets: QuestionSet[];
	isLoading: boolean;
	error: string | null;
}

const initialState: AdminStore = {
	media: [],
	questions: [],
	options: [],
	sets: [],
	isLoading: false,
	error: null
};

function createAdminStore() {
	const { subscribe, set, update } = writable<AdminStore>(initialState);

	return {
		subscribe,
		update: (updater: (state: AdminStore) => AdminStore) => update(updater),
		setData: (data: StoredData) => {
			update((state) => ({
				...state,
				media: data.media,
				questions: data.questions,
				options: data.options,
				sets: data.sets
			}));
		},
		setLoading: (loading: boolean) => {
			update((state) => ({ ...state, isLoading: loading }));
		},
		setError: (error: string | null) => {
			update((state) => ({ ...state, error }));
		},
		reset: () => set(initialState)
	};
}

export const adminStore = createAdminStore();
