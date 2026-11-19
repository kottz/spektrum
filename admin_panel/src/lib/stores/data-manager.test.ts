import { beforeEach, describe, expect, it } from 'vitest';
import { adminStore } from './data-manager.svelte';
import type { StoredData } from '#lib/generated/StoredData.ts';

const media = (id: number) => ({
	id,
	title: `Song ${id}`,
	artist: 'Artist',
	release_year: 2000,
	spotify_uri: null,
	youtube_id: 'yt'
});

function load(data: Partial<StoredData> = {}) {
	adminStore.setData({
		stored_data: { media: [], characters: [], questions: [], options: [], sets: [], ...data },
		version: 'v1'
	});
}

describe('admin data manager', () => {
	beforeEach(() => load());

	it('allocates ids at insertion, so redo cannot collide with a later add', () => {
		const first = adminStore.addEntity('media', { ...media(0), title: 'A' });
		adminStore.undo();
		// A form opened now would previously have pre-computed id `first`...
		adminStore.redo();
		const second = adminStore.addEntity('media', { ...media(0), title: 'B' });
		const ids = adminStore.getState().media.map((m) => m.id);
		expect(second).not.toBe(first);
		expect(new Set(ids).size).toBe(ids.length);
	});

	it('treats loaded data as a fresh start, discarding previous history', () => {
		adminStore.addEntity('media', media(0));
		load({ media: [media(7)] });
		expect(adminStore.canUndo()).toBe(false);
		expect(adminStore.getSnapshots()).toHaveLength(1);
		expect(adminStore.isLoaded()).toBe(true);
	});

	it('records a batch as a single undoable step', () => {
		adminStore.startBatch();
		adminStore.addEntity('media', media(0));
		adminStore.addEntity('media', media(0));
		adminStore.commitBatch();
		expect(adminStore.getState().media).toHaveLength(2);
		adminStore.undo();
		expect(adminStore.getState().media).toHaveLength(0);
	});

	it('exports data without client-only pending uploads', () => {
		const file = new File(['x'], 'abu.avif', { type: 'image/avif' });
		adminStore.addEntity('characters', {
			name: 'abu',
			image_url: 'img/abu.avif',
			_pendingImage: { dataUrl: 'data:', file }
		});
		const [character] = adminStore.exportData().characters;
		expect(character).not.toHaveProperty('_pendingImage');
		expect(character.name).toBe('abu');
	});
});

describe('admin data manager versioning', () => {
	it('bases edits on the version it last loaded or saved', () => {
		load();
		expect(adminStore.isLoaded()).toBe(true);
		expect(adminStore.getBaseVersion()).toBe('v1');

		adminStore.setData({ stored_data: adminStore.exportData(), version: 'v2' });
		expect(adminStore.getBaseVersion()).toBe('v2');
		expect(adminStore.canUndo()).toBe(false);
	});
});
