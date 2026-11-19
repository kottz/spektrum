import { describe, expect, it } from 'vitest';
import { questionProblem } from './question-rules';

const option = (option_text: string, is_correct = true) => ({ option_text, is_correct });

describe('questionProblem', () => {
	it('requires something to score as correct', () => {
		expect(questionProblem('color', null, [])).not.toBeNull();
		expect(questionProblem('color', null, [option('Red', false)])).not.toBeNull();
		expect(questionProblem('character', null, [option('mario', false)])).not.toBeNull();
		expect(questionProblem('color', null, [option('Red')])).toBeNull();
	});

	it('requires text questions to be fully written', () => {
		expect(questionProblem('text', '', [option('A')])).not.toBeNull();
		expect(questionProblem('text', 'Who?', [option('A'), option(' ', false)])).not.toBeNull();
		expect(questionProblem('text', 'Who?', [option('A'), option('B', false)])).toBeNull();
	});

	it('requires year questions to have exactly the correct year', () => {
		expect(questionProblem('year', null, [])).not.toBeNull();
		expect(questionProblem('year', null, [option('nineteen')])).not.toBeNull();
		expect(questionProblem('year', null, [option('1999', false)])).not.toBeNull();
		expect(questionProblem('year', null, [option('1999'), option('2000')])).not.toBeNull();
		expect(questionProblem('year', null, [option('1999')])).toBeNull();
	});
});
