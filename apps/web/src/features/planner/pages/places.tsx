import { useMemo } from "react";
import type { PlannerAppState, PageHelpers } from "./context";
import { Button, Icon } from "../../../shared/ui";

export function PlacesPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const {
    setModal,
    routeSection,
    goSpace,
    space,
    placeFilter,
    setPlaceFilter,
    places,
    placesStatus,
    placeError,
    retryPlaces,
  } = state;
  const { pageHeader } = helpers;
  const spacePlaces = useMemo(
    () => places.filter((place) => place.space === space.id),
    [places, space.id],
  );
  const categories = useMemo(
    () => ["All", ...new Set(spacePlaces.map((place) => place.category))],
    [spacePlaces],
  );
  const visiblePlaces = useMemo(
    () =>
      spacePlaces.filter(
        (place) => placeFilter === "All" || place.category === placeFilter,
      ),
    [spacePlaces, placeFilter],
  );
  const hasSavedPlaces = spacePlaces.length > 0;

  return (
    <>
      {pageHeader(
        "Saved places",
        `Places saved in ${space.name}. Only this Space's places are shown.`,
        hasSavedPlaces ? (
          <Button onClick={() => setModal("place")}>
            <Icon name="plus" />
            Save a place
          </Button>
        ) : undefined,
      )}
      <div className="space-page-tabs">
        {[
          ["calendar", "Calendar"],
          ["tasks", "To-dos"],
          ["places", "Places"],
          ["members", "Members"],
          ["settings", "Space settings"],
        ].map(([key, label]) => (
          <button
            key={key}
            className={routeSection === key ? "current" : ""}
            onClick={() => goSpace(space.id, key!)}
          >
            {label}
          </button>
        ))}
      </div>
      {placesStatus === "loading" && <p role="status">Loading Saved places…</p>}
      {placesStatus === "error" && (
        <div role="alert">
          <p>{placeError}</p>
          <Button variant="outline" onClick={retryPlaces}>
            Retry
          </Button>
        </div>
      )}
      <div className="place-controls">
        <div className="place-filters">
          {categories.map((category) => {
            const count =
              category === "All"
                ? spacePlaces.length
                : spacePlaces.filter((place) => place.category === category)
                    .length;
            return (
              <button
                className={placeFilter === category ? "picked" : ""}
                key={category}
                onClick={() => setPlaceFilter(category)}
              >
                {category} ({count})
              </button>
            );
          })}
        </div>
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
                  {place.id && (
                    <button
                      className="place-remove"
                      type="button"
                      disabled={state.removingPlaceId === place.id}
                      onClick={() => {
                        if (
                          window.confirm(
                            `Remove “${place.name}” from ${space.name}?`,
                          )
                        ) {
                          void state.removeSavedPlace(place);
                        }
                      }}
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
        placesStatus === "ready" && (
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
            {!hasSavedPlaces && (
              <Button variant="outline" onClick={() => setModal("place")}>
                Save a place
              </Button>
            )}
          </div>
        )
      )}
      {placeError && placesStatus !== "error" && (
        <p role="alert">{placeError}</p>
      )}
      {state.placesNextOffset !== null && (
        <Button
          variant="outline"
          disabled={state.loadingMorePlaces}
          onClick={state.loadMorePlaces}
        >
          {state.loadingMorePlaces ? "Loading…" : "Load more Saved places"}
        </Button>
      )}
    </>
  );
}
