import type { QuestionType } from '#lib/generated/QuestionType.ts';

interface OptionDraft {
	option_text: string;
	is_correct: boolean;
}

/** Year answers are whole years; the server offers the two years either side. */
const YEAR = /^\d{1,4}$/;

/**
 * Why a question could not be played as written, or null when it can. Mirrors
 * what the server needs to run a round: something to score as correct, and for
 * text and year questions, well-formed answers.
 */
export function questionProblem(
	type: QuestionType,
	questionText: string | null,
	options: readonly OptionDraft[]
): string | null {
	const correct = options.filter((o) => o.is_correct);
	switch (type) {
		case 'color':
		case 'character':
			return correct.length === 0 ? 'Mark at least one correct option.' : null;
		case 'text':
			if (!questionText?.trim()) return 'Text questions need question text.';
			if (options.some((o) => !o.option_text.trim())) return 'Options cannot be empty.';
			return correct.length === 0 ? 'Mark at least one correct option.' : null;
		case 'year':
			if (options.length !== 1 || !correct[0] || !YEAR.test(correct[0].option_text.trim())) {
				return 'Year questions need the correct year.';
			}
			return null;
	}
}
