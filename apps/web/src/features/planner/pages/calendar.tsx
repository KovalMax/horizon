import { useEffect, useRef } from "react";
import type { PlannerAppState, PageHelpers } from "./context";
import { dateLabel } from "../domain";
import { Button, Icon, SpaceGlyph } from "../../../shared/ui";

export function CalendarPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const {
    setModal,
    space,
    events,
    calendarDay,
    setCalendarDay,
    eventsStatus,
    eventError,
    retryEvents,
  } = state;
  const { pageHeader, eventRow } = helpers;
  const canManage = space.role !== "Viewer";
  const dateStripRef = useRef<HTMLDivElement>(null);
  const dateForOffset = (offset: number) => {
    const date = new Date();
    date.setHours(12, 0, 0, 0);
    date.setDate(date.getDate() + offset);
    return date;
  };
  const dateValue = (offset: number) => {
    const date = dateForOffset(offset);
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
  };
  const monthLabel = dateForOffset(calendarDay).toLocaleDateString(undefined, {
    month: "long",
    year: "numeric",
  });
  const dayOffsets = Array.from({ length: 195 }, (_, index) => index - 14);
  const selectedEvents = events.filter(
    (event) => event.space === space.id && event.day === calendarDay,
  );
  const comingUp = dayOffsets
    .filter((day) => day > calendarDay)
    .map((day) => ({
      day,
      events: events.filter(
        (event) => event.space === space.id && event.day === day,
      ),
    }))
    .filter(({ events: dayEvents }) => dayEvents.length > 0)
    .slice(0, 4);
  useEffect(() => {
    const strip = dateStripRef.current;
    const selectedDate = strip?.querySelector<HTMLButtonElement>(
      '[aria-pressed="true"]',
    );
    if (!strip || !selectedDate) return;
    const scrollLeft =
      strip.scrollLeft +
      selectedDate.getBoundingClientRect().left -
      strip.getBoundingClientRect().left -
      (strip.clientWidth - selectedDate.offsetWidth) / 2;
    if (typeof strip.scrollTo === "function") {
      strip.scrollTo({ left: scrollLeft, behavior: "auto" });
    } else {
      strip.scrollLeft = scrollLeft;
    }
  }, [calendarDay]);
  return (
    <>
      <div className="space-context-tag">
        <SpaceGlyph space={space} small />
        {space.name}
      </div>
      {pageHeader(
        "Calendar",
        "Times follow each event’s time zone",
        canManage ? (
          <Button onClick={() => setModal("event")}>
            <Icon name="plus" />
            New event
          </Button>
        ) : null,
      )}
      {!canManage && (
        <p className="read-only-notice">
          You can view Calendar events in this Space, but only Editors and the
          Owner can change them.
        </p>
      )}
      <div className="calendar-layout">
        <div className="calendar-main-column">
          <section className="calendar-board" aria-label="Space calendar">
            <div className="calendar-toolbar">
              <label className="calendar-date-picker">
                <strong>{monthLabel}</strong>
                <span aria-hidden="true">⌄</span>
                <span className="visually-hidden">Choose a date</span>
                <input
                  aria-label="Choose a date"
                  type="date"
                  value={dateValue(calendarDay)}
                  min={dateValue(-14)}
                  max={dateValue(180)}
                  onChange={(event) => {
                    if (!event.target.value) return;
                    const selectedDate = new Date(
                      `${event.target.value}T12:00:00`,
                    );
                    const today = dateForOffset(0);
                    setCalendarDay(
                      Math.round(
                        (selectedDate.getTime() - today.getTime()) / 86_400_000,
                      ),
                    );
                  }}
                />
              </label>
              <div className="calendar-toolbar-actions">
                <Button
                  variant="outline"
                  disabled={calendarDay === 0}
                  onClick={() => setCalendarDay(0)}
                >
                  Today
                </Button>
                <Button
                  variant="quiet"
                  aria-label="Previous week"
                  disabled={calendarDay <= -14}
                  onClick={() => setCalendarDay(Math.max(-14, calendarDay - 7))}
                >
                  ‹
                </Button>
                <Button
                  variant="quiet"
                  aria-label="Next week"
                  disabled={calendarDay >= 180}
                  onClick={() => setCalendarDay(Math.min(180, calendarDay + 7))}
                >
                  ›
                </Button>
              </div>
            </div>
            <div
              ref={dateStripRef}
              className="calendar-date-strip"
              aria-label="Choose a date"
              role="group"
              onKeyDown={(event) => {
                const steps: Record<string, number> = {
                  ArrowLeft: -1,
                  ArrowRight: 1,
                  PageUp: -7,
                  PageDown: 7,
                };
                if (event.key === "Home") {
                  event.preventDefault();
                  dateStripRef.current
                    ?.querySelector<HTMLButtonElement>('[data-day="0"]')
                    ?.focus();
                  setCalendarDay(0);
                  return;
                }
                const step = steps[event.key];
                if (step === undefined) return;
                event.preventDefault();
                const nextDay = Math.max(
                  -14,
                  Math.min(180, calendarDay + step),
                );
                dateStripRef.current
                  ?.querySelector<HTMLButtonElement>(`[data-day="${nextDay}"]`)
                  ?.focus();
                setCalendarDay(nextDay);
              }}
            >
              {dayOffsets.map((day) => {
                const date = dateForOffset(day);
                const dayEvents = events.filter(
                  (event) => event.space === space.id && event.day === day,
                );
                const isSelected = calendarDay === day;
                const isToday = day === 0;
                return (
                  <div className="calendar-date-item" key={day}>
                    {date.getDate() === 1 && (
                      <span
                        aria-hidden="true"
                        className="calendar-month-divider"
                      >
                        {date.toLocaleDateString(undefined, { month: "short" })}
                      </span>
                    )}
                    <button
                      data-day={day}
                      aria-pressed={isSelected}
                      tabIndex={isSelected ? 0 : -1}
                      aria-current={isToday ? "date" : undefined}
                      aria-label={`${dateLabel(day, true)}${isToday ? ", today" : ""}, ${dayEvents.length} ${dayEvents.length === 1 ? "event" : "events"}`}
                      className={[
                        isSelected ? "selected" : "",
                        isToday ? "today" : "",
                        day < 0 ? "past" : "",
                        !isSelected &&
                        day >= 0 &&
                        [0, 6].includes(date.getDay())
                          ? "weekend"
                          : "",
                      ]
                        .filter(Boolean)
                        .join(" ")}
                      onClick={() => setCalendarDay(day)}
                    >
                      <span>
                        {date.toLocaleDateString(undefined, {
                          weekday: "short",
                        })}
                      </span>
                      <b>{date.getDate()}</b>
                      <i aria-hidden="true">
                        {Array.from(
                          { length: Math.min(dayEvents.length, 3) },
                          (_, index) => (
                            <i key={index} />
                          ),
                        )}
                      </i>
                    </button>
                  </div>
                );
              })}
            </div>
            <section className="calendar-selected-day" aria-live="polite">
              <div className="calendar-section-heading">
                <div>
                  <h2>
                    {calendarDay === 0
                      ? "Today"
                      : calendarDay === 1
                        ? "Tomorrow"
                        : dateForOffset(calendarDay).toLocaleDateString(
                            undefined,
                            { weekday: "long" },
                          )}
                    <span>
                      {" "}
                      · {dateLabel(calendarDay, true).replace(/^[^ ]+ /, "")}
                    </span>
                  </h2>
                  <p>
                    {selectedEvents.length === 0
                      ? "Nothing planned"
                      : `${selectedEvents.length} ${selectedEvents.length === 1 ? "event" : "events"}`}
                  </p>
                </div>
                <div className="calendar-toolbar-actions">
                  <Button
                    variant="quiet"
                    aria-label="Previous day"
                    disabled={calendarDay <= -14}
                    onClick={() =>
                      setCalendarDay(Math.max(-14, calendarDay - 1))
                    }
                  >
                    ‹
                  </Button>
                  <Button
                    variant="quiet"
                    aria-label="Next day"
                    disabled={calendarDay >= 180}
                    onClick={() =>
                      setCalendarDay(Math.min(180, calendarDay + 1))
                    }
                  >
                    ›
                  </Button>
                </div>
              </div>
              {selectedEvents.length ? (
                <div className="calendar-selected-events">
                  {selectedEvents.map((event) => eventRow(event))}
                </div>
              ) : (
                <div className="calendar-free-day">
                  <span>
                    A free day{calendarDay < 0 ? "" : " so far"} — nothing on
                    the {space.name} calendar.
                  </span>
                  {canManage && calendarDay >= 0 && (
                    <Button variant="outline" onClick={() => setModal("event")}>
                      <Icon name="plus" /> Add on{" "}
                      {dateLabel(calendarDay, true).split(",")[0]}
                    </Button>
                  )}
                </div>
              )}
              {canManage && selectedEvents.length > 0 && calendarDay >= 0 && (
                <Button variant="quiet" onClick={() => setModal("event")}>
                  <Icon name="plus" /> Add another on{" "}
                  {dateLabel(calendarDay, true).split(",")[0]}
                </Button>
              )}
            </section>
          </section>
          <section className="calendar-coming-up" aria-labelledby="coming-up">
            <h2 id="coming-up">
              Coming up
              {calendarDay > 0 && (
                <span> after {dateLabel(calendarDay, true).split(",")[0]}</span>
              )}
            </h2>
            {comingUp.length ? (
              comingUp.map(({ day, events: dayEvents }) => (
                <div className="coming-up-day" key={day}>
                  <button onClick={() => setCalendarDay(day)}>
                    {day === 1 ? "Tomorrow" : dateLabel(day, true)}
                  </button>
                  {dayEvents.map((event) => eventRow(event))}
                </div>
              ))
            ) : (
              <p className="calendar-empty-upcoming">
                No more events in the next few weeks.
              </p>
            )}
          </section>
        </div>
        <aside className="tip-card calendar-tip">
          <b>About times</b>
          <p>
            Each event is shown in the time zone it was scheduled in. Open an
            event to see its time zone.
          </p>
          <p>All-day events stay on the same date for everyone.</p>
          <p>Use the arrow keys in the day strip to move between days.</p>
        </aside>
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
    </>
  );
}
