import { describe, expect, it } from 'vitest';
import { getResponseError } from './http';

describe('getResponseError', () => {
	it('uses API error details when available', async () => {
		const response = new Response(JSON.stringify({ details: 'Lobby is full.' }), {
			status: 400,
			headers: { 'content-type': 'application/json' }
		});

		await expect(getResponseError(response, 'Failed to join lobby')).resolves.toBe(
			'Lobby is full.'
		);
	});

	it('falls back to the HTTP status for malformed responses', async () => {
		const response = new Response('<html>upstream unavailable</html>', {
			status: 503,
			statusText: 'Service Unavailable',
			headers: { 'content-type': 'text/html' }
		});

		await expect(getResponseError(response, 'Failed to join lobby')).resolves.toBe(
			'Failed to join lobby (503 Service Unavailable)'
		);
	});
});
