import { authenticatedFetch } from "../auth/api";
import type { Space } from "./domain";

type ApiSpace = {
  id: string;
  name: string;
  kind: "personal" | "shared";
  role: "owner" | "editor" | "viewer";
  member_count: number;
};

export type DeletedSpace = {
  id: string;
  name: string;
  kind: "personal" | "shared";
  deleted_at: string;
  restore_until: string;
};

export class SpacesApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code?: string,
  ) {
    super("Spaces could not be loaded or saved.");
  }
}

async function throwSpacesApiError(response: Response): Promise<never> {
  let code: string | undefined;
  try {
    const problem: unknown = await response.json();
    if (
      typeof problem === "object" &&
      problem !== null &&
      "code" in problem &&
      typeof problem.code === "string"
    ) {
      code = problem.code;
    }
  } catch {
    // Some error responses, such as gateway failures, have no JSON body.
  }
  throw new SpacesApiError(response.status, code);
}

function toSpace(space: ApiSpace): Space {
  const personal = space.kind === "personal";
  return {
    id: space.id,
    name: space.name,
    role: (space.role[0]!.toUpperCase() + space.role.slice(1)) as Space["role"],
    members: space.member_count,
    tone: personal ? "personal" : "home",
    icon: personal ? "person" : "home",
  };
}

export async function listSpaces(accessToken: string): Promise<Space[]> {
  const response = await authenticatedFetch("/api/spaces", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) await throwSpacesApiError(response);
  const result = (await response.json()) as ApiSpace[];
  return result.map(toSpace);
}

export async function createSharedSpace(
  accessToken: string,
  name: string,
): Promise<Space> {
  const response = await authenticatedFetch("/api/spaces", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ name }),
  });
  if (!response.ok) await throwSpacesApiError(response);
  return toSpace((await response.json()) as ApiSpace);
}

export async function transferSpaceOwnership(
  accessToken: string,
  spaceId: string,
  newOwnerId: string,
): Promise<void> {
  const response = await authenticatedFetch(`/api/spaces/${spaceId}/owner`, {
    method: "PATCH",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ new_owner_id: newOwnerId }),
  });
  if (!response.ok) await throwSpacesApiError(response);
}

export async function deleteSpace(
  accessToken: string,
  spaceId: string,
): Promise<DeletedSpace> {
  const response = await authenticatedFetch(`/api/spaces/${spaceId}`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) await throwSpacesApiError(response);
  return (await response.json()) as DeletedSpace;
}

export async function listDeletedSpaces(
  accessToken: string,
): Promise<DeletedSpace[]> {
  const response = await authenticatedFetch("/api/spaces/deleted", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) await throwSpacesApiError(response);
  return (await response.json()) as DeletedSpace[];
}

export async function restoreSpace(
  accessToken: string,
  spaceId: string,
): Promise<Space> {
  const response = await authenticatedFetch(`/api/spaces/${spaceId}/restore`, {
    method: "POST",
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) await throwSpacesApiError(response);
  return toSpace((await response.json()) as ApiSpace);
}
