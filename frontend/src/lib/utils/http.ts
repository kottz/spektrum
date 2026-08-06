interface ApiErrorBody {
	error?: unknown;
	details?: unknown;
}

export async function getResponseError(response: Response, fallback: string): Promise<string> {
	const contentType = response.headers.get('content-type') ?? '';

	try {
		if (contentType.includes('application/json')) {
			const body = (await response.json()) as ApiErrorBody;
			if (typeof body.details === 'string' && body.details) return body.details;
			if (typeof body.error === 'string' && body.error) return body.error;
		} else if (contentType.startsWith('text/plain')) {
			const message = (await response.text()).trim();
			if (message && message.length <= 200) return message;
		}
	} catch {
		// The status below is more useful than a secondary response parsing error.
	}

	const status = `${response.status} ${response.statusText}`.trim();
	return status ? `${fallback} (${status})` : fallback;
}
