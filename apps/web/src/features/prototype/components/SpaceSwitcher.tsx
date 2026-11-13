import type { PrototypeAppState } from "../usePrototypeApp";
import { Icon, SpaceGlyph } from "../../../shared/ui";

export function SpaceSwitcher({ appState }: { appState: PrototypeAppState }) {
  const {
    spaceSwitcherOpen,
    setSpaceSwitcherOpen,
    isSpacePage,
    space,
    routeSection,
    go,
    goSpace,
    setModal,
    activeSpaces,
  } = appState;
  if (!spaceSwitcherOpen) return null;

  const sections = ["calendar", "tasks", "places", "members", "settings"];
  const targetSection = sections.includes(routeSection)
    ? routeSection
    : "calendar";

  return (
    <div
      className="drawer-backdrop"
      onClick={() => setSpaceSwitcherOpen(false)}
    >
      <section
        aria-labelledby="space-switcher-title"
        aria-modal="true"
        className="more-drawer space-switcher"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <div className="drawer-heading">
          <h2 id="space-switcher-title">Switch Space</h2>
          <button
            aria-label="Close Space switcher"
            onClick={() => setSpaceSwitcherOpen(false)}
          >
            ×
          </button>
        </div>
        <nav aria-label="Spaces" className="switcher-options">
          <button
            aria-current={!isSpacePage ? "page" : undefined}
            className={`switcher-option ${!isSpacePage ? "current" : ""}`}
            onClick={() => go("/")}
          >
            <SpaceGlyph />
            <span className="switcher-option-copy">
              <b>All Spaces</b>
              <small>Events and To-dos from every Space</small>
            </span>
            {!isSpacePage && <span className="switcher-current">Current</span>}
          </button>
          {activeSpaces.map((item) => {
            const current = isSpacePage && item.id === space.id;
            return (
              <button
                key={item.id}
                aria-current={current ? "page" : undefined}
                className={`switcher-option ${current ? "current" : ""}`}
                onClick={() => goSpace(item.id, targetSection)}
              >
                <SpaceGlyph space={item} />
                <span className="switcher-option-copy">
                  <b>{item.name}</b>
                  <small>
                    {item.role} · {item.members}{" "}
                    {item.members === 1 ? "member" : "members"}
                  </small>
                </span>
                {current && <span className="switcher-current">Current</span>}
              </button>
            );
          })}
          {appState.spacesStatus === "loading" && (
            <p role="status">Loading Spaces…</p>
          )}
          {appState.spacesStatus === "error" && (
            <div role="alert">
              <p>{appState.spacesError}</p>
              <button onClick={appState.retrySpaces}>Retry</button>
            </div>
          )}
        </nav>
        <button
          className="switcher-create"
          onClick={() => {
            setSpaceSwitcherOpen(false);
            setModal("space");
          }}
        >
          <Icon name="plus" />
          New shared Space
        </button>
      </section>
    </div>
  );
}
