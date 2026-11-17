import type { PlannerAppState, PageHelpers } from "./context";
import { dateLabel, todayLong, type Space } from "../domain";
import { Button, Icon, SpaceGlyph } from "../../../shared/ui";

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
  const { pageHeader, spaceCards, taskRow, eventRow } = helpers;
  const noSpacesVisible = activeSpaces.every((item) =>
    hidden.includes(item.id),
  );
  const undatedTasks = filteredTasks.filter((task) => task.day === undefined);
  return (
    <>
      {pageHeader(
        "Your Spaces",
        "Everything coming up across your account Spaces.",
        <Button onClick={() => setModal("event")}>
          <Icon name="plus" />
          New event
        </Button>,
      )}
      {state.myInvitations.length > 0 && (
        <section className="list-card" aria-label="Space invitations">
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
      {spaceCards}
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
                  h.includes(s.id) ? h.filter((v) => v !== s.id) : [...h, s.id],
                )
              }
            >
              <SpaceGlyph space={s} small />
              {s.name}
            </button>
          ))}
        </div>
        <div className="segmented">
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
      <div className="overview-grid">
        <div className="timeline">
          {noSpacesVisible ? (
            <section className="list-card empty-overview" role="status">
              <Icon name="layers" />
              <div>
                <h2>All Spaces are filtered out</h2>
                <p>Turn a Space back on above to see its events and To-dos.</p>
              </div>
            </section>
          ) : filteredEvents.length === 0 && filteredTasks.length === 0 ? (
            <section className="list-card empty-overview" role="status">
              <Icon name="calendar" />
              <div>
                <h2>Nothing coming up</h2>
                <p>
                  There are no events or open To-dos to show with these filters.
                </p>
              </div>
            </section>
          ) : (
            days.map((day) => {
              const ev = filteredEvents.filter((e) => e.day === day);
              const ts = filteredTasks.filter((t) => t.day === day);
              if (!ev.length && !ts.length) return null;
              return (
                <section className="day-card" key={day}>
                  <h2>
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
                  </h2>
                  {ev.map((e) => eventRow(e, true))}
                  {ev.length > 0 && ts.length > 0 && <hr />}
                  {ts.map((t) => taskRow(t, true))}
                </section>
              );
            })
          )}
        </div>
        <aside className="overview-aside">
          {undatedTasks.length > 0 && (
            <section className="day-card undated">
              <h2>No due date</h2>
              {undatedTasks.map((task) => taskRow(task, true))}
            </section>
          )}
          <div className="note-card">
            <Icon name="pin" />
            <p>
              Saved places belong to one Space. Open a Space to see its saved
              places.
            </p>
          </div>
        </aside>
      </div>
      {state.eventError && state.eventsStatus !== "error" && (
        <p role="alert">{state.eventError}</p>
      )}
      {state.taskError && state.tasksStatus !== "error" && (
        <p role="alert">{state.taskError}</p>
      )}
      {state.eventsNextOffset !== null && (
        <Button
          variant="outline"
          disabled={state.loadingMoreEvents}
          onClick={state.loadMoreEvents}
        >
          {state.loadingMoreEvents ? "Loading…" : "Load more events"}
        </Button>
      )}
      {state.tasksNextOffset !== null && (
        <Button
          variant="outline"
          disabled={state.loadingMoreTasks}
          onClick={state.loadMoreTasks}
        >
          {state.loadingMoreTasks ? "Loading…" : "Load more Tasks"}
        </Button>
      )}
    </>
  );
}
