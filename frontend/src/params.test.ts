import { describe, expect, it } from 'vitest';
import { JOIN_CODE } from './params';

describe('join code matcher', () => {
	it('accepts every code length the server issues', () => {
		expect(JOIN_CODE.test('123456')).toBe(true);
		expect(JOIN_CODE.test('1234567')).toBe(true);
	});

	it('rejects anything else', () => {
		for (const code of ['12345', '12345678', '12a456', '']) {
			expect(JOIN_CODE.test(code)).toBe(false);
		}
	});
});
