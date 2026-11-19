import { PUBLIC_SPEKTRUM_SERVER_URL } from '$app/env/public';
import { getResponseError } from '#lib/http.ts';
import type { GetStoredDataRequest } from '#lib/generated/GetStoredDataRequest.ts';
import type { SetStoredDataRequest } from '#lib/generated/SetStoredDataRequest.ts';
import type { StoredDataResponse } from '#lib/generated/StoredDataResponse.ts';
import type { UploadCharacterImageResponse } from '#lib/generated/UploadCharacterImageResponse.ts';

/** A failed request, carrying the server's explanation and the HTTP status. */
export class ApiError extends Error {
	constructor(
		message: string,
		readonly status: number
	) {
		super(message);
	}
}

/**
 * POSTs to a server endpoint. Request and response shapes are generated from
 * the server's types; a JSON body is encoded, form data is sent as is.
 */
async function post<Res>(path: string, failure: string, body: FormData | object): Promise<Res> {
	const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}${path}`, {
		method: 'POST',
		cache: 'no-store',
		...(body instanceof FormData
			? { body }
			: { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
	});
	if (!response.ok) {
		throw new ApiError(await getResponseError(response, failure), response.status);
	}
	return (await response.json()) as Res;
}

export const api = {
	getStoredData: (req: GetStoredDataRequest) =>
		post<StoredDataResponse>('/api/questions', 'Failed to load data', req),
	setStoredData: (req: SetStoredDataRequest) =>
		post<StoredDataResponse>('/api/update-questions', 'Failed to apply changes', req),
	uploadCharacterImage: (name: string, image: File, password: string) => {
		const form = new FormData();
		form.append('image', image);
		form.append('password', password);
		return post<UploadCharacterImageResponse>(
			`/api/upload-character-image/${encodeURIComponent(name)}`,
			`Failed to upload ${name}`,
			form
		);
	}
};
