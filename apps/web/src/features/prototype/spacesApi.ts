import type { Space } from "./domain";

type ApiSpace = {
  id: string;
  name: string;
  kind: "personal" | "shared";
  role: "owner" | "editor" | "viewer";
  member_count: number;
};

export class SpacesApiError extends Error {
  constructor(public readonly status: number) {
    super("Spaces could not be loaded or saved.");
  }
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
  const response = await fetch("/api/spaces", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) throw new SpacesApiError(response.status);
  const result = (await response.json()) as ApiSpace[];
  return result.map(toSpace);
}

export async function createSharedSpace(
  accessToken: string,
  name: string,
): Promise<Space> {
  const response = await fetch("/api/spaces", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ name }),
  });
  if (!response.ok) throw new SpacesApiError(response.status);
  return toSpace((await response.json()) as ApiSpace);
}
