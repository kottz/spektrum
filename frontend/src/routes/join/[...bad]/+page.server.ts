// src/routes/join/[...bad]/+page.server.ts
import { error } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export function load() {
	throw error(404, 'Invalid lobby code');
}
