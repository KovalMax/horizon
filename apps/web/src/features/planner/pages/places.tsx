import type { PlannerAppState, PageHelpers } from "./context";
import { Button, Icon } from "../../../shared/ui";
import { useSpacePlaces } from "../useSpacePlaces";

export function PlacesPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const { setModal, space, accessToken, placesRevision, placeError } = state;
  const { pageHeader } = helpers;
  const canManage = Boolean(space.id) && space.role !== "Viewer";
  const {
    category,
    setCategory,
    counts,
    page,
    error,
    retry,
    pageError,
    loadingMore,
    loadMore,
  } = useSpacePlaces(accessToken, space.id, placesRevision);
  const total = Object.values(counts ?? {}).reduce(
    (sum, count) => sum + count,
    0,
  );
  const categories = Array.from(
    new Set([...Object.keys(counts ?? {}), ...(category ? [category] : [])]),
  ).sort();
  const visiblePlaces = page?.items ?? [];
  const hasSavedPlaces = total > 0;

  return (
    <>
      {pageHeader(
        "Saved places",
        `Places saved in ${space.name}. Only this Space's places are shown.`,
        hasSavedPlaces && canManage ? (
          <Button onClick={() => setModal("place")}>
            <Icon name="plus" />
            Save a place
          </Button>
        ) : undefined,
      )}
      {!canManage && (
        <p className="read-only-notice">
          You can view Saved places in this Space, but only Editors and the
          Owner can change them.
        </p>
      )}
      {!page && !error && <p role="status">Loading Saved places…</p>}
      {error && (
        <div role="alert">
          <p>{error}</p>
          <Button variant="outline" onClick={retry}>
            Retry
          </Button>
        </div>
      )}
      <div className="place-controls">
        {counts && (
          <div
            className="place-filters"
            role="group"
            aria-label="Place categories"
          >
            {[null, ...categories].map((option) => (
              <button
                type="button"
                className={category === option ? "picked" : ""}
                aria-pressed={category === option}
                key={option ?? "all"}
                onClick={() => setCategory(option)}
              >
                {option ?? "All"} ({option ? (counts[option] ?? 0) : total})
              </button>
            ))}
          </div>
        )}
        {hasSavedPlaces && (
          <div
            className="place-view-toggle"
            role="group"
            aria-label="Place view"
          >
            <button type="button" aria-pressed="true">
              <Icon name="list" size={16} />
              List
            </button>
            <button
              type="button"
              disabled
              title="Map view is coming soon."
              aria-label="Map view coming soon"
            >
              <Icon name="map" size={16} />
              Map
            </button>
          </div>
        )}
      </div>
      {visiblePlaces.length > 0 ? (
        <div className="saved-places-list">
          {visiblePlaces.map((place) => (
            <article className="saved-place-card" key={place.id ?? place.name}>
              <div className="place-thumb">
                <Icon name="pin" size={22} />
              </div>
              <div className="saved-place-content">
                <div className="saved-place-title">
                  <h2>{place.name}</h2>
                  <span className="place-category-tag">{place.category}</span>
                </div>
                <p>{place.address}</p>
                {place.note && <p className="place-note">{place.note}</p>}
                <div className="saved-place-actions">
                  <a
                    className="place-directions"
                    href={`https://www.google.com/maps/dir/?api=1&destination=${encodeURIComponent(place.address)}`}
                    target="_blank"
                    rel="noreferrer"
                  >
                    <Icon name="arrow" size={16} />
                    Directions
                  </a>
                  {canManage && place.id && (
                    <button
                      className="place-remove"
                      type="button"
                      disabled={state.removingPlaceId === place.id}
                      onClick={() =>
                        state.requestConfirmation({
                          title: `Remove “${place.name}”?`,
                          description: `This will remove the Saved place from ${space.name}.`,
                          confirmLabel: "Remove Saved place",
                          onConfirm: () => state.removeSavedPlace(place),
                        })
                      }
                    >
                      {state.removingPlaceId === place.id
                        ? "Removing…"
                        : "Remove"}
                    </button>
                  )}
                </div>
              </div>
            </article>
          ))}
        </div>
      ) : (
        page && (
          <div className="empty-state">
            <h2>
              {hasSavedPlaces
                ? "No places in this category"
                : "No saved places yet"}
            </h2>
            <p>
              {hasSavedPlaces
                ? "Choose another category to see more places."
                : `Save a useful place for everyone in ${space.name}.`}
            </p>
            {!hasSavedPlaces && canManage && (
              <Button variant="outline" onClick={() => setModal("place")}>
                Save a place
              </Button>
            )}
          </div>
        )
      )}
      {placeError && <p role="alert">{placeError}</p>}
      {pageError && <p role="alert">{pageError}</p>}
      {page && page.nextOffset !== null && (
        <Button variant="outline" disabled={loadingMore} onClick={loadMore}>
          {loadingMore ? "Loading…" : "Load more Saved places"}
        </Button>
      )}
    </>
  );
}
