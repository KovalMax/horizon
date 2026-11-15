import { useMemo } from "react";
import type { PlannerAppState, PageHelpers } from "./context";
import { Button, Icon } from "../../../shared/ui";

const placeCategories = [
  "All",
  "Food & drink",
  "Groceries",
  "Health",
  "Services",
];

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
  const visiblePlaces = useMemo(
    () =>
      places.filter(
        (place) =>
          place.space === space.id &&
          (placeFilter === "All" || place.category === placeFilter),
      ),
    [places, space.id, placeFilter],
  );
  const hasSavedPlaces = places.some((place) => place.space === space.id);

  return (
    <>
      {pageHeader(
        "Saved places",
        `Places saved in ${space.name}. Only this Space's places are shown.`,
        <Button onClick={() => setModal("place")}>
          <Icon name="plus" />
          Save a place
        </Button>,
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
      <div className="place-filters">
        {placeCategories.map((category) => (
          <button
            className={placeFilter === category ? "picked" : ""}
            key={category}
            onClick={() => setPlaceFilter(category)}
          >
            {category}{" "}
            {category === "All"
              ? `(${places.filter((place) => place.space === space.id).length})`
              : ""}
          </button>
        ))}
      </div>
      {visiblePlaces.length > 0 ? (
        <div className="places-grid">
          {visiblePlaces.map((place) => (
            <article className="place-card" key={place.id ?? place.name}>
              <div className="place-thumb">
                <Icon name="pin" size={22} />
              </div>
              <div>
                <span className="eyebrow">{place.category}</span>
                <h2>{place.name}</h2>
                <p>{place.address}</p>
                {place.note && <p className="place-note">{place.note}</p>}
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
            <Button variant="outline" onClick={() => setModal("place")}>
              Save a place
            </Button>
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
