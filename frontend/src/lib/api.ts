import { PUBLIC_SPEKTRUM_SERVER_URL } from '$app/env/public';
import { getResponseError } from '#lib/utils/http.ts';
import type { CheckSessionsRequest } from '#lib/generated/CheckSessionsRequest.ts';
import type { CheckSessionsResponse } from '#lib/generated/CheckSessionsResponse.ts';
import type { CreateLobbyRequest } from '#lib/generated/CreateLobbyRequest.ts';
import type { CreateLobbyResponse } from '#lib/generated/CreateLobbyResponse.ts';
import type { JoinLobbyRequest } from '#lib/generated/JoinLobbyRequest.ts';
import type { JoinLobbyResponse } from '#lib/generated/JoinLobbyResponse.ts';
import type { ListSetsResponse } from '#lib/generated/ListSetsResponse.ts';

/**
 * Calls a server endpoint, throwing an Error with the server's explanation when
 * it fails. Request and response shapes are generated from the server's types.
 */
async function request<Res>(path: string, failure: string, body?: unknown): Promise<Res> {
	const response = await fetch(`${PUBLIC_SPEKTRUM_SERVER_URL}${path}`, {
		cache: 'no-store',
		...(body === undefined
			? {}
			: {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify(body)
				})
	});
	if (!response.ok) {
		throw new Error(await getResponseError(response, failure));
	}
	return (await response.json()) as Res;
}

export const api = {
	listSets: () => request<ListSetsResponse>('/api/list-sets', 'Failed to load question sets'),
	createLobby: (req: CreateLobbyRequest) =>
		request<CreateLobbyResponse>('/api/create-lobby', 'Failed to create lobby', req),
	joinLobby: (req: JoinLobbyRequest) =>
		request<JoinLobbyResponse>('/api/join-lobby', 'Failed to join lobby', req),
	checkSessions: (req: CheckSessionsRequest) =>
		request<CheckSessionsResponse>('/api/check-sessions', 'Failed to check sessions', req)
};
