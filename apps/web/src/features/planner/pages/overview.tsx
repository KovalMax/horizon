import type { PlannerAppState, PageHelpers } from "./context";
import { dateLabel, todayLong, type Space } from "../domain";
import { AppLink, Button, Icon, SpaceGlyph } from "../../../shared/ui";

export function OverviewPage({
  state,
  helpers,
  activeSpaces,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
  activeSpaces: Space[];
}) {
  const {
    setModal,
    hidden,
    setHidden,
    filter,
    setFilter,
    days,
    filteredEvents,
    filteredTasks,
  } = state;
  const { pageHeader, taskRow, eventRow } = helpers;
  const noSpacesVisible =
    activeSpaces.length > 0 &&
    activeSpaces.every((item) => hidden.includes(item.id));
  const undatedTasks = filteredTasks.filter((task) => task.day === undefined);
  const timelineDays = days
    .map((day) => ({
      day,
      events: filteredEvents.filter((event) => event.day === day),
      tasks: filteredTasks.filter((task) => task.day === day),
    }))
    .filter(({ events, tasks }) => events.length > 0 || tasks.length > 0);
  const timelineSources = [
    { status: state.spacesStatus, retry: state.retrySpaces },
    ...(filter !== "tasks"
      ? [{ status: state.eventsStatus, retry: state.retryEvents }]
      : []),
    ...(filter !== "events"
      ? [{ status: state.tasksStatus, retry: state.retryTasks }]
      : []),
  ];
  const failedSources = timelineSources.filter(
    ({ status }) => status === "error",
  );
  const timelineLoading = timelineSources.some(
    ({ status }) => status === "idle" || status === "loading",
  );
  return (
    <>
      {pageHeader(
        "Your plans",
        "See what’s coming up across your Spaces.",
        <Button onClick={() => setModal("event")}>
          <Icon name="plus" />
          New event
        </Button>,
      )}
      <section className="overview-section" aria-labelledby="upcoming-heading">
        <h2 id="upcoming-heading" className="overview-heading">
          Today &amp; coming up
        </h2>
        <div className="overview-controls">
          <div
            className="space-filters"
            role="group"
            aria-label="Filter by Space"
          >
            {activeSpaces.map((s) => (
              <button
                key={s.id}
                aria-pressed={!hidden.includes(s.id)}
                className={`filter-pill ${s.tone} ${hidden.includes(s.id) ? "off" : ""}`}
                onClick={() =>
                  setHidden((h) =>
                    h.includes(s.id)
                      ? h.filter((v) => v !== s.id)
                      : [...h, s.id],
                  )
                }
              >
                <SpaceGlyph space={s} small />
                {s.name}
              </button>
            ))}
          </div>
          <div
            className="segmented"
            role="group"
            aria-label="Filter by item type"
          >
            {[
              ["all", "Everything"],
              ["events", "Events"],
              ["tasks", "To-dos"],
            ].map(([key, label]) => (
              <button
                key={key}
                aria-pressed={filter === key}
                onClick={() => setFilter(key!)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
        <div className="timeline">
          {noSpacesVisible ? (
            <section className="list-card empty-overview" role="status">
              <Icon name="layers" />
              <div>
                <h3>All Spaces are filtered out</h3>
                <p>Turn a Space back on above to see its events and To-dos.</p>
              </div>
            </section>
          ) : timelineDays.length === 0 && failedSources.length > 0 ? (
            <section className="list-card">
              <p role="alert">The timeline couldn’t be loaded.</p>
              <Button
                variant="outline"
                onClick={() => failedSources.forEach(({ retry }) => retry())}
              >
                Retry timeline
              </Button>
            </section>
          ) : timelineDays.length === 0 && timelineLoading ? (
            <p role="status">Loading timeline…</p>
          ) : timelineDays.length === 0 ? (
            <section className="list-card empty-overview" role="status">
              <Icon name="calendar" />
              <div>
                <h3>Nothing scheduled in the next two weeks</h3>
              </div>
            </section>
          ) : (
            timelineDays.map(({ day, events, tasks }) => (
              <section className="day-card" key={day}>
                <h3>
                  {day === 0 ? (
                    <>
                      <b>Today</b> · {todayLong}
                    </>
                  ) : day === 1 ? (
                    <>
                      <b>Tomorrow</b> · {dateLabel(day, true)}
                    </>
                  ) : (
                    <b>{dateLabel(day, true)}</b>
                  )}
                </h3>
                {events.map((e) => eventRow(e, true))}
                {events.length > 0 && tasks.length > 0 && <hr />}
                {tasks.map((t) => taskRow(t, true))}
              </section>
            ))
          )}
        </div>
        {state.eventError && state.eventsStatus !== "error" && (
          <p role="alert">{state.eventError}</p>
        )}
        {state.taskError && state.tasksStatus !== "error" && (
          <p role="alert">{state.taskError}</p>
        )}
        {filter !== "tasks" && state.eventsNextOffset !== null && (
          <Button
            variant="outline"
            disabled={state.loadingMoreEvents}
            onClick={state.loadMoreEvents}
          >
            {state.loadingMoreEvents ? "Loading…" : "Load more events"}
          </Button>
        )}
        {filter !== "events" && state.tasksNextOffset !== null && (
          <Button
            variant="outline"
            disabled={state.loadingMoreTasks}
            onClick={state.loadMoreTasks}
          >
            {state.loadingMoreTasks ? "Loading…" : "Load more Tasks"}
          </Button>
        )}
      </section>
      {undatedTasks.length > 0 && (
        <section className="overview-section" aria-labelledby="undated-heading">
          <h2 id="undated-heading" className="overview-heading">
            No due date
          </h2>
          <div className="day-card undated">
            {undatedTasks.map((task) => taskRow(task, true))}
          </div>
        </section>
      )}
      {state.myInvitations.length > 0 && (
        <section
          className="list-card overview-section"
          aria-label="Space invitations"
        >
          <h2>Invitations for you</h2>
          <p>Accept an invitation to add a Space to your account.</p>
          {state.myInvitations.map((invitation) => (
            <div className="invite-row" key={invitation.id}>
              <span>
                <b>{invitation.space_name}</b>
                <small>{invitation.email} · Viewer</small>
              </span>
              <Button
                disabled={state.acceptingInvitation === invitation.id}
                onClick={() => state.acceptInvitation(invitation.id)}
              >
                {state.acceptingInvitation === invitation.id
                  ? "Joining…"
                  : "Accept"}
              </Button>
            </div>
          ))}
        </section>
      )}
      {activeSpaces.length > 0 && (
        <section
          className="overview-section"
          aria-labelledby="your-spaces-heading"
        >
          <h2 id="your-spaces-heading" className="overview-heading">
            Your Spaces
          </h2>
          <div className="space-cards">
            {activeSpaces.map((space) => (
              <AppLink
                key={space.id}
                to={`/spaces/${space.id}/calendar`}
                onNavigate={state.go}
                className={`space-card ${space.tone}`}
              >
                <SpaceGlyph space={space} />
                <strong>{space.name}</strong>
                <Icon name="arrow" size={16} />
              </AppLink>
            ))}
          </div>
        </section>
      )}
    </>
  );
}
