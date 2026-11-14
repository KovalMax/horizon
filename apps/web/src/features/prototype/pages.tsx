import type { ReactNode } from "react";
import { type Space } from "./domain";
import type { PrototypeAppState } from "./usePrototypeApp";
import { EventRow, PageHeader, TaskRow } from "./components/TimelineRows";
import { AppLink, Icon, SpaceGlyph } from "../../shared/ui";
import {
  CalendarPage,
  MembersPage,
  NotificationsPage,
  OverviewPage,
  PlacesPage,
  SettingsPage,
  SpaceSettingsPage,
  TasksPage,
  WelcomePage,
} from "./pages/index";
import type { PageHelpers } from "./pages/context";

function buildPageHelpers(
  state: PrototypeAppState,
  activeSpaces: Space[],
): PageHelpers {
  const pageHeader: PageHelpers["pageHeader"] = (
    title,
    description,
    action,
  ) => <PageHeader title={title} description={description} action={action} />;
  const spaceCards = (
    <section aria-label="Your Spaces" className="space-cards">
      {activeSpaces.map((item) => {
        const eventCount = state.events.filter(
          (event) => event.space === item.id,
        ).length;
        const openTodoCount = state.tasks.filter(
          (task) => task.space === item.id && !task.done,
        ).length;
        return (
          <AppLink
            key={item.id}
            to={`/spaces/${item.id}/calendar`}
            onNavigate={state.go}
            className={`space-card ${item.tone}`}
          >
            <div className="space-card-head">
              <SpaceGlyph space={item} />
              <div>
                <b>{item.name}</b>
                <small>
                  {item.role} ·{" "}
                  {item.members +
                    (item.id === "home" && state.approved ? 1 : 0)}{" "}
                  {item.members +
                    (item.id === "home" && state.approved ? 1 : 0) ===
                  1
                    ? "member"
                    : "members"}
                </small>
              </div>
            </div>
            <p>
              {eventCount} {eventCount === 1 ? "event" : "events"} in 2 weeks ·{" "}
              {openTodoCount} open {openTodoCount === 1 ? "To-do" : "To-dos"}
            </p>
            <strong>
              Open Space <Icon name="arrow" size={16} />
            </strong>
          </AppLink>
        );
      })}
    </section>
  );
  const taskRow: PageHelpers["taskRow"] = (task, withSpace = false) => (
    <TaskRow
      key={`task-${task.id}`}
      task={task}
      withSpace={withSpace}
      canToggle={
        !state.accessToken ||
        activeSpaces.find((space) => space.id === task.space)?.role !== "Viewer"
      }
      onToggle={state.toggleTask}
      onSelect={state.setSelected}
    />
  );
  const eventRow: PageHelpers["eventRow"] = (event, withSpace = false) => (
    <EventRow
      key={`event-${event.id}-${event.occurrenceDate ?? "single"}`}
      event={event}
      withSpace={withSpace}
      onSelect={state.setSelected}
    />
  );
  return { pageHeader, spaceCards, taskRow, eventRow };
}

export function renderPageContent(
  state: PrototypeAppState,
  activeSpaces = state.activeSpaces,
): ReactNode {
  const helpers = buildPageHelpers(state, activeSpaces);
  const { path, isSpacePage, routeSection } = state;
  if (path === "/welcome") return <WelcomePage state={state} />;
  if (path === "/notifications")
    return <NotificationsPage state={state} helpers={helpers} />;
  if (path === "/settings")
    return <SettingsPage state={state} helpers={helpers} />;
  if (!isSpacePage)
    return (
      <OverviewPage
        state={state}
        helpers={helpers}
        activeSpaces={activeSpaces}
      />
    );
  if (routeSection === "tasks")
    return <TasksPage state={state} helpers={helpers} />;
  if (routeSection === "places")
    return <PlacesPage state={state} helpers={helpers} />;
  if (routeSection === "members")
    return <MembersPage state={state} helpers={helpers} />;
  if (routeSection === "settings")
    return <SpaceSettingsPage state={state} helpers={helpers} />;
  return <CalendarPage state={state} helpers={helpers} />;
}
