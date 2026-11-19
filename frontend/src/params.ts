import { defineParams } from '@sveltejs/kit/params';

/** Join codes are 6 digits, or 7 once 6-digit codes are crowded (see generate_join_code). */
export const JOIN_CODE = /^\d{6,7}$/;

export const params = defineParams({
	joinCode: (value) => (JOIN_CODE.test(value) ? value : undefined)
});
