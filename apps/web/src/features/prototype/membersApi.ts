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
): Promise<SpaceMember[]> {
  return readJson(
    await fetch(`/api/spaces/${spaceId}/members`, {
      headers: headers(accessToken),
    }),
  );
}

export async function listSpaceInvitations(
  accessToken: string,
  spaceId: string,
): Promise<SpaceInvitation[]> {
  return readJson(
    await fetch(`/api/spaces/${spaceId}/invitations`, {
      headers: headers(accessToken),
    }),
  );
}

export async function createSpaceInvitation(
  accessToken: string,
  spaceId: string,
  email: string,
): Promise<SpaceInvitation> {
  return readJson(
    await fetch(`/api/spaces/${spaceId}/invitations`, {
      method: "POST",
      headers: headers(accessToken, true),
      body: JSON.stringify({ email }),
    }),
  );
}

export async function listMyInvitations(
  accessToken: string,
): Promise<SpaceInvitation[]> {
  return readJson(
    await fetch("/api/invitations/current", {
      headers: headers(accessToken),
    }),
  );
}

export async function acceptSpaceInvitation(
  accessToken: string,
  invitationId: string,
): Promise<SpaceInvitation> {
  return readJson(
    await fetch(`/api/invitations/${invitationId}/accept`, {
      method: "POST",
      headers: headers(accessToken),
    }),
  );
}
