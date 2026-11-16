import { renderPageContent } from "./pages";
import type { PlannerAppState } from "./usePlannerApp";
import {
  AppLink,
  BottomBar,
  HorizonMark,
  Icon,
  SpaceGlyph,
} from "../../shared/ui";
import { PlannerOverlays } from "./components/PlannerOverlays";
import { AuthScreen } from "../auth/AuthScreen";
import type { AuthState } from "../auth/useAuth";
export function PlannerShell({
  appState,
  authState,
}: {
  appState: PlannerAppState;
  authState: AuthState;
}) {
  if (authState.restoring) {
    return (
      <main className="welcome-card auth-card" role="status">
        Restoring your Horizon session…
      </main>
    );
  }
  const {
    ready,
    activeSpaces,
    spacesStatus,
    spacesError,
    path,
    unread,
    mobileMenu,
    setMobileMenu,
    spaceSwitcherOpen,
    setSpaceSwitcherOpen,
    setModal,
    space,
    isSpacePage,
    routeSection,
    go,
    goSpace,
  } = appState;
  if (path === "/welcome" || path === "/auth/complete") {
    return (
      <AuthScreen
        path={path}
        auth={authState}
        navigate={go}
        joinRequests={appState.myJoinRequests}
        requestJoinAccess={appState.requestJoinAccess}
      />
    );
  }
  const sidebar = (
    <aside className="sidebar">
      <div className="brand-row">
        <AppLink to="/" onNavigate={go} className="brand">
          <HorizonMark />
          <b>Horizon</b>
        </AppLink>
      </div>
      <nav className="primary-nav" aria-label="Overview">
        <AppLink
          to="/"
          onNavigate={go}
          className={`nav-link ${path === "/" ? "active" : ""}`}
        >
          <Icon name="layers" />
          All Spaces
        </AppLink>
        <AppLink
          to="/notifications"
          onNavigate={go}
          className={`nav-link ${path === "/notifications" ? "active" : ""}`}
        >
          <Icon name="bell" />
          Notifications{unread > 0 && <span className="count">{unread}</span>}
        </AppLink>
        <AppLink
          to="/settings"
          onNavigate={go}
          className={`nav-link ${path === "/settings" ? "active" : ""}`}
        >
          <Icon name="settings" />
          Settings
        </AppLink>
      </nav>
      <nav aria-label="Spaces" className="space-nav">
        <h2>Your Spaces</h2>
        {spacesStatus === "loading" && <p role="status">Loading Spaces…</p>}
        {spacesStatus === "error" && (
          <div role="alert">
            <p>{spacesError}</p>
            <button className="nav-link" onClick={appState.retrySpaces}>
              Retry
            </button>
          </div>
        )}
        {activeSpaces.map((s) => (
          <div
            key={s.id}
            className={`space-nav-entry ${space.id === s.id && isSpacePage ? "selected" : ""}`}
          >
            <button className="space-nav-button" onClick={() => goSpace(s.id)}>
              <SpaceGlyph space={s} />
              <span>
                <b>{s.name}</b>
                <small>{s.role}</small>
              </span>
              {space.id === s.id && isSpacePage && (
                <i className={`selected-dot ${s.tone}`} />
              )}
            </button>
            {space.id === s.id && isSpacePage && (
              <div className="subnav">
                {[
                  ["calendar", "calendar", "Calendar"],
                  ["tasks", "list", "To-dos"],
                  ["places", "pin", "Places"],
                  ["members", "users", "Members"],
                  ["settings", "settings", "Space settings"],
                ].map(([section, icon, label]) => (
                  <AppLink
                    key={section}
                    to={`/spaces/${s.id}/${section}`}
                    onNavigate={go}
                    className={`nav-link sublink ${routeSection === section ? "active" : ""}`}
                  >
                    <Icon name={icon!} />
                    {label}
                  </AppLink>
                ))}
              </div>
            )}
          </div>
        ))}
        <button className="nav-link dashed" onClick={() => setModal("space")}>
          <Icon name="plus" />
          New shared Space
        </button>
      </nav>
      <div className="sidebar-bottom">
        {authState.account ? (
          <div className="account-summary">
            <small>Signed in as</small>
            <span>{authState.account.email}</span>
            <button
              className="nav-link"
              onClick={() => {
                void authState.signOut().then(() => go("/welcome"));
              }}
            >
              Sign out
            </button>
          </div>
        ) : (
          <AppLink to="/welcome" onNavigate={go} className="nav-link">
            Sign in
          </AppLink>
        )}
        {ready !== "available" && (
          <div
            className="api-state"
            role={ready === "unavailable" ? "alert" : "status"}
            aria-live={ready === "unavailable" ? "assertive" : "polite"}
          >
            {ready === "checking"
              ? "Checking service…"
              : "Service is unavailable."}
          </div>
        )}
      </div>
    </aside>
  );

  const content = renderPageContent(appState, activeSpaces);

  const isOverview = path === "/";
  return (
    <div className="app-shell">
      {sidebar}
      <div className="main-column">
        <header className="mobile-header">
          <button
            className="mobile-space-switch"
            aria-expanded={spaceSwitcherOpen}
            aria-haspopup="dialog"
            onClick={() => setSpaceSwitcherOpen(!spaceSwitcherOpen)}
          >
            <SpaceGlyph space={isSpacePage ? space : undefined} />
            <span>
              <small>{isSpacePage ? "Space" : "Showing"}</small>
              <b>{isSpacePage ? space.name : "All Spaces"}</b>
            </span>
            <Icon name="chevron-down" size={16} />
            <span className="visually-hidden">Switch Space</span>
          </button>
          <div>
            <AppLink
              to="/notifications"
              onNavigate={go}
              className="mobile-icon"
              ariaLabel={`Notifications, ${unread} unread`}
            >
              <Icon name="bell" />
              {unread > 0 && <i />}
            </AppLink>
          </div>
        </header>
        <main
          className={`main-content ${isOverview ? "overview-content" : ""}`}
        >
          {authState.error && <p role="alert">{authState.error}</p>}
          {content}
        </main>
        <BottomBar
          path={path}
          space={space}
          go={go}
          goSpace={goSpace}
          onMore={() => setMobileMenu(true)}
        />
      </div>
      <PlannerOverlays appState={appState} />
    </div>
  );
}
