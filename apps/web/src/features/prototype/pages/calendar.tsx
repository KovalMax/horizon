import type { PrototypeAppState, PageHelpers } from "./context";
import { dateLabel } from "../domain";
import { Button, Icon, SpaceGlyph } from "../../../shared/ui";

export function CalendarPage({
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
    events,
    calendarDay,
    setCalendarDay,
    calendarExpanded,
    setCalendarExpanded,
    eventsStatus,
    eventError,
    retryEvents,
  } = state;
  const { pageHeader, eventRow } = helpers;
  return (
    <>
      {pageHeader(
        "Calendar",
        "Times shown in your time · Berlin (CET)",
        <Button onClick={() => setModal("event")}>
          <Icon name="plus" />
          New event
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
      <div className="space-title">
        <SpaceGlyph space={space} />
        {space.name}
      </div>
      <div
        className="calendar-date-strip"
        aria-label="Choose a date"
        role="group"
      >
        {Array.from({ length: 7 }, (_, index) => index + calendarDay).map(
          (day) => (
            <button
              aria-pressed={calendarDay === day}
              key={day}
              onClick={() => setCalendarDay(day)}
            >
              <span>{dateLabel(day).split(" ")[0]}</span>
              <b>
                {new Date(
                  new Date().setDate(new Date().getDate() + day),
                ).getDate()}
              </b>
            </button>
          ),
        )}
      </div>
      {eventsStatus === "loading" && (
        <p role="status">Loading Calendar events…</p>
      )}
      {eventsStatus === "error" && (
        <div role="alert">
          <p>{eventError}</p>
          <Button variant="outline" onClick={retryEvents}>
            Retry
          </Button>
        </div>
      )}
      <div className="calendar-day-list">
        {Array.from(
          { length: calendarExpanded ? 35 : 7 },
          (_, index) => index + calendarDay,
        ).map((day) => {
          const list = events.filter(
            (e) => e.space === space.id && e.day === day,
          );
          if (!list.length) return null;
          return (
            <section className="day-card" key={day}>
              <h2>
                {day === 0 ? "Today · " : day === 1 ? "Tomorrow · " : ""}
                {dateLabel(day, true)}
              </h2>
              {list.map((e) => eventRow(e))}
            </section>
          );
        })}
      </div>
      {eventError && eventsStatus !== "error" && (
        <p role="alert">{eventError}</p>
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
      <Button
        variant="outline"
        onClick={() => setCalendarExpanded(!calendarExpanded)}
      >
        {calendarExpanded ? "Show the next week" : "Show 4 more weeks"}
      </Button>
      <aside className="tip-card">
        <b>About times</b>
        <p>
          Every event keeps the timezone it was created in. You see it converted
          to Berlin (CET); open an event to see the original time and each
          member's local time.
        </p>
        <p>All-day events stay on the same date for everyone.</p>
      </aside>
    </>
  );
}
