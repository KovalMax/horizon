import { authenticatedFetch } from "../auth/api";
export type SpaceMember = {
  user_id: string;
  email: string;
  role: "owner" | "editor" | "viewer";
};

export type SpaceInvitation = {
  id: string;
  space_id: string;
  space_name: string;
  email: string;
  role: "viewer";
  expires_at: string;
};

export type SpaceJoinLink = {
  id: string;
  space_id: string;
  space_name: string;
  role: "viewer" | "editor";
  created_at: string;
  expires_at: string | null;
  revoked_at: string | null;
  url: string;
};

export type SpaceJoinRequest = {
  id: string;
  link_id: string;
  space_id: string;
  space_name: string;
  requester_id: string;
  requester_email: string | null;
  role: "viewer" | "editor";
  status: "pending" | "approved" | "declined";
  requested_at: string;
  decided_at: string | null;
};

export class MembersApiError extends Error {
  constructor(public readonly status: number) {
    super("Space membership could not be loaded or changed.");
  }
}

async function readJson<T>(response: Response): Promise<T> {
  if (!response.ok) throw new MembersApiError(response.status);
  return (await response.json()) as T;
}

function headers(accessToken: string, json = false): HeadersInit {
  return {
    Authorization: `Bearer ${accessToken}`,
    ...(json ? { "Content-Type": "application/json" } : {}),
  };
}

export async function listSpaceMembers(
  accessToken: string,
  spaceId: string,
  signal?: AbortSignal,
): Promise<SpaceMember[]> {
  return readJson(
    await authenticatedFetch(`/api/spaces/${spaceId}/members`, {
      headers: headers(accessToken),
      signal,
    }),
  );
}

export async function listSpaceInvitations(
  accessToken: string,
  spaceId: string,
  signal?: AbortSignal,
): Promise<SpaceInvitation[]> {
  return readJson(
    await authenticatedFetch(`/api/spaces/${spaceId}/invitations`, {
      headers: headers(accessToken),
      signal,
    }),
  );
}

export async function createSpaceInvitation(
  accessToken: string,
  spaceId: string,
  email: string,
): Promise<SpaceInvitation> {
  return readJson(
    await authenticatedFetch(`/api/spaces/${spaceId}/invitations`, {
      method: "POST",
      headers: headers(accessToken, true),
      body: JSON.stringify({ email }),
    }),
  );
}

export async function updateSpaceMemberRole(
  accessToken: string,
  spaceId: string,
  userId: string,
  role: "editor" | "viewer",
): Promise<SpaceMember> {
  return readJson(
    await authenticatedFetch(`/api/spaces/${spaceId}/members/${userId}`, {
      method: "PATCH",
      headers: headers(accessToken, true),
      body: JSON.stringify({ role }),
    }),
  );
}

export async function removeSpaceMember(
  accessToken: string,
  spaceId: string,
  userId: string,
): Promise<void> {
  const response = await authenticatedFetch(
    `/api/spaces/${spaceId}/members/${userId}`,
    {
      method: "DELETE",
      headers: headers(accessToken),
    },
  );
  if (!response.ok) throw new MembersApiError(response.status);
}

export async function revokeSpaceInvitation(
  accessToken: string,
  spaceId: string,
  invitationId: string,
): Promise<void> {
  const response = await authenticatedFetch(
    `/api/spaces/${spaceId}/invitations/${invitationId}`,
    { method: "DELETE", headers: headers(accessToken) },
  );
  if (!response.ok) throw new MembersApiError(response.status);
}

export async function listMyInvitations(
  accessToken: string,
  signal?: AbortSignal,
): Promise<SpaceInvitation[]> {
  return readJson(
    await authenticatedFetch("/api/invitations/current", {
      headers: headers(accessToken),
      signal,
    }),
  );
}

export async function acceptSpaceInvitation(
  accessToken: string,
  invitationId: string,
): Promise<SpaceInvitation> {
  return readJson(
    await authenticatedFetch(`/api/invitations/${invitationId}/accept`, {
      method: "POST",
      headers: headers(accessToken),
    }),
  );
}

export async function listSpaceJoinLinks(
  accessToken: string,
  spaceId: string,
  signal?: AbortSignal,
): Promise<SpaceJoinLink[]> {
  return readJson(
    await authenticatedFetch(`/api/spaces/${spaceId}/join-links`, {
      headers: headers(accessToken),
      signal,
    }),
  );
}

export async function createSpaceJoinLink(
  accessToken: string,
  spaceId: string,
  role: "viewer" | "editor",
  expiresAt: string | null,
): Promise<SpaceJoinLink> {
  return readJson(
    await authenticatedFetch(`/api/spaces/${spaceId}/join-links`, {
      method: "POST",
      headers: headers(accessToken, true),
      body: JSON.stringify({ role, expires_at: expiresAt }),
    }),
  );
}

export async function revokeSpaceJoinLink(
  accessToken: string,
  spaceId: string,
  linkId: string,
): Promise<void> {
  const response = await authenticatedFetch(
    `/api/spaces/${spaceId}/join-links/${linkId}`,
    {
      method: "DELETE",
      headers: headers(accessToken),
    },
  );
  if (!response.ok) throw new MembersApiError(response.status);
}

export async function listPendingSpaceJoinRequests(
  accessToken: string,
  spaceId: string,
  signal?: AbortSignal,
): Promise<SpaceJoinRequest[]> {
  return readJson(
    await authenticatedFetch(`/api/spaces/${spaceId}/join-requests`, {
      headers: headers(accessToken),
      signal,
    }),
  );
}

export async function decideSpaceJoinRequest(
  accessToken: string,
  spaceId: string,
  requestId: string,
  decision: "approved" | "declined",
): Promise<SpaceJoinRequest> {
  return readJson(
    await authenticatedFetch(
      `/api/spaces/${spaceId}/join-requests/${requestId}/decision`,
      {
        method: "POST",
        headers: headers(accessToken, true),
        body: JSON.stringify({ decision }),
      },
    ),
  );
}

export async function requestSpaceJoin(
  accessToken: string,
  linkId: string,
): Promise<SpaceJoinRequest> {
  return readJson(
    await authenticatedFetch(`/api/join-links/${linkId}/requests`, {
      method: "POST",
      headers: headers(accessToken),
    }),
  );
}

export async function listMySpaceJoinRequests(
  accessToken: string,
  signal?: AbortSignal,
): Promise<SpaceJoinRequest[]> {
  return readJson(
    await authenticatedFetch("/api/join-requests/current", {
      headers: headers(accessToken),
      signal,
    }),
  );
}
