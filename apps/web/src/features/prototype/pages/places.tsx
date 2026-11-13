import type { PrototypeAppState, PageHelpers } from "./context";
import { Button, Icon } from "../../../shared/ui";

export function PlacesPage({
  state,
  helpers,
}: {
  state: PrototypeAppState;
  helpers: PageHelpers;
}) {
  const {
    setModal,
    routeSection,
    goSpace,
    space,
    placeFilter,
    setPlaceFilter,
    mapMode,
    setMapMode,
    showToast,
    places,
    placesStatus,
    placeError,
    retryPlaces,
  } = state;
  const { pageHeader } = helpers;
  const visiblePlaces = places.filter(
    (place) =>
      (!state.accessToken ||
        state.activeSpaces.some((item) => item.id === place.space)) &&
      place.space === space.id &&
      (placeFilter === "All" || place.category === placeFilter),
  );
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
      <div className="place-controls">
        <div className="place-filters">
          {["All", "Food & drink", "Groceries", "Health", "Services"].map(
            (f) => (
              <button
                className={placeFilter === f ? "picked" : ""}
                key={f}
                onClick={() => setPlaceFilter(f)}
              >
                {f}{" "}
                {f === "All"
                  ? `(${places.filter((place) => place.space === space.id).length})`
                  : ""}
              </button>
            ),
          )}
        </div>
        <div className="segmented">
          <button aria-pressed={!mapMode} onClick={() => setMapMode(false)}>
            List
          </button>
          <button aria-pressed={mapMode} onClick={() => setMapMode(true)}>
            Map
          </button>
        </div>
      </div>
      {mapMode ? (
        <div
          className="map-canvas"
          aria-label="Illustrative map, not connected to a map service"
        >
          <span className="map-disclaimer">
            Illustrative only · no live map data
          </span>
          {visiblePlaces.map((p, i) => (
            <button
              key={p.id ?? p.name}
              className="map-pin"
              style={{ left: `${17 + i * 15}%`, top: `${20 + (i % 3) * 22}%` }}
              onClick={() => showToast(p.name)}
              title={p.name}
            >
              <Icon name="pin" size={22} />
            </button>
          ))}
          <span className="map-label one">KREUZBERG</span>
          <span className="map-label two">Landwehrkanal</span>
        </div>
      ) : (
        <div className="places-grid">
          {visiblePlaces.map((p) => (
            <article className="place-card" key={p.id ?? p.name}>
              <div className="place-thumb">
                <Icon name="pin" size={22} />
              </div>
              <div>
                <span className="eyebrow">{p.category}</span>
                <h2>{p.name}</h2>
                <p>{p.address}</p>
                {p.note && <p className="place-note">{p.note}</p>}
                <button
                  className="directions-link"
                  onClick={() =>
                    showToast("Directions aren't connected in this prototype.")
                  }
                >
                  Directions
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
      {!visiblePlaces.length && state.placesNextOffset === null && (
        <div className="empty-state">
          <h2>No saved places yet</h2>
          <p>Save a useful place for everyone in {space.name}.</p>
          <Button variant="outline" onClick={() => setModal("place")}>
            Save a place
          </Button>
        </div>
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
