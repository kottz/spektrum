// src/routes/join/[code]/+page.ts
export const prerender = false; // render client‑side only

/** @type {import('./$types').PageLoad} */
export function load({ params }) {
	return { joinCode: params.code };
}
