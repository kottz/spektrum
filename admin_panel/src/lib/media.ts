import { PUBLIC_SPEKTRUM_CDN_URL } from '$app/env/public';
import type { Character } from '#lib/types.ts';

/** Resolves a stored media path (e.g. `img/abu.avif`) against the CDN, when one is configured. */
export function mediaUrl(path: string): string {
	return PUBLIC_SPEKTRUM_CDN_URL ? `${PUBLIC_SPEKTRUM_CDN_URL}/${path}` : path;
}

/** A pending upload previews from its data URL; stored images resolve through the CDN. */
export function characterImageSrc(character: Pick<Character, 'image_url' | '_pendingImage'>) {
	return character._pendingImage?.dataUrl ?? mediaUrl(character.image_url);
}
