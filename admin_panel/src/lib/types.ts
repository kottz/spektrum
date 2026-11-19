import type { Character as StoredCharacter } from '#lib/generated/Character.ts';
import type { Color } from '#lib/generated/Color.ts';
import type { QuestionType } from '#lib/generated/QuestionType.ts';

/* Editor-side types; the stored data's own types are generated from the server in #lib/generated. */

/** A character as edited here, possibly carrying an image that is not uploaded yet. */
export interface Character extends StoredCharacter {
	_pendingImage?: {
		dataUrl: string;
		file: File;
	};
}

/** Every question type, in display order. The record makes the list exhaustive. */
export const QUESTION_TYPES = Object.keys({
	color: null,
	character: null,
	text: null,
	year: null
} satisfies Record<QuestionType, null>) as QuestionType[];

/** Every answer color, in display order. The record makes the list exhaustive. */
export const COLORS = Object.keys({
	Red: null,
	Green: null,
	Blue: null,
	Yellow: null,
	Purple: null,
	Gold: null,
	Silver: null,
	Pink: null,
	Black: null,
	White: null,
	Brown: null,
	Orange: null,
	Gray: null
} satisfies Record<Color, null>) as Color[];
