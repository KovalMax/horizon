import { authenticatedFetch } from "../auth/api";
import type { PlaceItem } from "./domain";

type ApiSavedPlace = {
  id: string;
  space_id: string;
  space_name: string;
  space_kind: "personal" | "shared";
  name: string;
  category: string;
  address: string;
  note: string | null;
};

export class PlacesApiError extends Error {
  constructor(public readonly status: number) {
    super("Saved places could not be loaded or saved.");
  }
}

type ApiPlacePage = {
  items: ApiSavedPlace[];
  next_offset: number | null;
};

function toPlace(place: ApiSavedPlace): PlaceItem {
  return {
    id: place.id,
    name: place.name,
    category: place.category,
    address: place.address,
    note: place.note ?? undefined,
    space: place.space_id,
    spaceName: place.space_name,
    spaceTone: place.space_kind === "personal" ? "personal" : "home",
  };
}

async function readJson<T>(response: Response): Promise<T> {
  if (!response.ok) throw new PlacesApiError(response.status);
  return (await response.json()) as T;
}

export async function listSavedPlaces(
  accessToken: string,
  offset = 0,
): Promise<{ items: PlaceItem[]; nextOffset: number | null }> {
  const page = await readJson<ApiPlacePage>(
    await authenticatedFetch(`/api/places?offset=${offset}`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    }),
  );
  return {
    items: page.items.map(toPlace),
    nextOffset: page.next_offset,
  };
}

export async function createSavedPlace(
  accessToken: string,
  spaceId: string,
  place: Omit<PlaceItem, "id" | "space">,
): Promise<PlaceItem> {
  const saved = await readJson<ApiSavedPlace>(
    await authenticatedFetch(`/api/spaces/${spaceId}/places`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        name: place.name,
        category: place.category,
        address: place.address,
        note: place.note ?? null,
      }),
    }),
  );
  return toPlace(saved);
}

export async function deleteSavedPlace(
  accessToken: string,
  spaceId: string,
  placeId: string,
): Promise<void> {
  const response = await authenticatedFetch(
    `/api/spaces/${spaceId}/places/${placeId}`,
    {
      method: "DELETE",
      headers: { Authorization: `Bearer ${accessToken}` },
    },
  );
  if (!response.ok) throw new PlacesApiError(response.status);
}
