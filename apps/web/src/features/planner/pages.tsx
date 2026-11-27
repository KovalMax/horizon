import type { ReactNode } from "react";
import { type Space } from "./domain";
import type { PlannerAppState } from "./usePlannerApp";
import { EventRow, PageHeader, TaskRow } from "./components/TimelineRows";
import {
  CalendarPage,
  MembersPage,
  NotificationsPage,
  OverviewPage,
  PlacesPage,
  SettingsPage,
  SpaceSettingsPage,
  TasksPage,
} from "./pages/index";
import type { PageHelpers } from "./pages/context";

function buildPageHelpers(
  state: PlannerAppState,
  activeSpaces: Space[],
): PageHelpers {
  const pageHeader: PageHelpers["pageHeader"] = (
    title,
    description,
    action,
  ) => <PageHeader title={title} description={description} action={action} />;
  const taskRow: PageHelpers["taskRow"] = (task, withSpace = false) => (
    <TaskRow
      key={`task-${task.id}`}
      task={task}
      withSpace={withSpace}
      canToggle={
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
  return { pageHeader, taskRow, eventRow };
}

export function renderPageContent(
  state: PlannerAppState,
  account: { email: string; signOut: () => void } | null,
): ReactNode {
  const { activeSpaces } = state;
  const helpers = buildPageHelpers(state, activeSpaces);
  const { path, isSpacePage, routeSection } = state;
  if (path === "/notifications")
    return <NotificationsPage state={state} helpers={helpers} />;
  if (path === "/settings")
    return <SettingsPage state={state} helpers={helpers} account={account} />;
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
    return <PlacesPage key={state.space.id} state={state} helpers={helpers} />;
  if (routeSection === "members")
    return <MembersPage state={state} helpers={helpers} />;
  if (routeSection === "settings")
    return <SpaceSettingsPage state={state} helpers={helpers} />;
  return <CalendarPage state={state} helpers={helpers} />;
}
