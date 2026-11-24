import { useEffect, useRef, useState } from "react";
import type { PlannerAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

const supportedTimeZones: string[] =
  typeof Intl !== "undefined" && "supportedValuesOf" in Intl
    ? (
        Intl as typeof Intl & {
          supportedValuesOf: (key: "timeZone") => string[];
        }
      ).supportedValuesOf("timeZone")
    : [
        "UTC",
        "America/Los_Angeles",
        "America/Chicago",
        "America/New_York",
        "Europe/London",
        "Europe/Berlin",
        "Europe/Helsinki",
        "Asia/Tokyo",
        "Australia/Sydney",
      ];
function createTimeZoneLabel(zone: string): string {
  const name =
    zone === "UTC" ? "UTC" : zone.split("/").at(-1)!.replaceAll("_", " ");
  const offset =
    new Intl.DateTimeFormat("en", {
      timeZone: zone,
      timeZoneName: "shortOffset",
    })
      .formatToParts(new Date())
      .find((part) => part.type === "timeZoneName")?.value ?? "GMT";
  return `${name} — ${offset}`;
}

const timeZones = [
  "UTC",
  ...supportedTimeZones.filter((zone) => zone !== "UTC"),
].map((zone) => ({ value: zone, label: createTimeZoneLabel(zone) }));

export function SettingsPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const {
    reminderTime,
    setReminderTime,
    timeZone,
    setTimeZone,
    notificationPrefs,
    setNotificationPrefs,
    notificationPrefsStatus,
    mutedSpaces,
    setMutedSpaces,
  } = state;
  const [timeZoneSearch, setTimeZoneSearch] = useState("");
  const [timeZonePickerOpen, setTimeZonePickerOpen] = useState(false);
  const timeZonePickerRef = useRef<HTMLDivElement>(null);
  const selectedTimeZone = timeZones.find((zone) => zone.value === timeZone);
  const visibleTimeZones = timeZones.filter((zone) =>
    `${zone.label} ${zone.value}`
      .toLocaleLowerCase()
      .includes(timeZoneSearch.trim().toLocaleLowerCase()),
  );
  useEffect(() => {
    if (!timeZonePickerOpen) return;
    const closeOnOutsideClick = (event: MouseEvent) => {
      if (
        event.target instanceof Node &&
        !timeZonePickerRef.current?.contains(event.target)
      ) {
        setTimeZonePickerOpen(false);
        setTimeZoneSearch("");
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      setTimeZonePickerOpen(false);
      setTimeZoneSearch("");
    };
    document.addEventListener("click", closeOnOutsideClick);
    document.addEventListener("keydown", closeOnEscape, true);
    return () => {
      document.removeEventListener("click", closeOnOutsideClick);
      document.removeEventListener("keydown", closeOnEscape, true);
    };
  }, [timeZonePickerOpen]);
  const { pageHeader } = helpers;
  return (
    <>
      {pageHeader(
        "Settings",
        "Manage your time, reminders, and notification preferences.",
      )}
      <section className="list-card settings-time-fields">
        <h2>Time and reminders</h2>
        <p>Changes are saved with the Save settings button below.</p>
        <div className="form-field">
          <label htmlFor="time-zone-search">Time zone</label>
          <div className="time-zone-picker" ref={timeZonePickerRef}>
            <input
              id="time-zone-search"
              type="search"
              role="combobox"
              aria-autocomplete="list"
              aria-expanded={timeZonePickerOpen}
              aria-controls="time-zone-options"
              value={
                timeZonePickerOpen
                  ? timeZoneSearch
                  : (selectedTimeZone?.label ?? timeZone)
              }
              onFocus={() => {
                setTimeZoneSearch("");
                setTimeZonePickerOpen(true);
              }}
              onClick={() => setTimeZonePickerOpen(true)}
              onChange={(event) => {
                setTimeZoneSearch(event.target.value);
                setTimeZonePickerOpen(true);
              }}
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  setTimeZonePickerOpen(false);
                  setTimeZoneSearch("");
                } else if (event.key === "Enter" && visibleTimeZones[0]) {
                  event.preventDefault();
                  setTimeZone(visibleTimeZones[0].value);
                  setTimeZonePickerOpen(false);
                  setTimeZoneSearch("");
                }
              }}
              placeholder="Search city, region, or GMT offset"
              autoComplete="off"
            />
            {timeZonePickerOpen && (
              <div
                className="time-zone-options"
                id="time-zone-options"
                role="listbox"
                aria-label="Time zones"
              >
                {visibleTimeZones.length ? (
                  visibleTimeZones.map((zone) => (
                    <button
                      className="time-zone-option"
                      type="button"
                      role="option"
                      aria-selected={zone.value === timeZone}
                      key={zone.value}
                      onMouseDown={(event) => event.preventDefault()}
                      onClick={() => {
                        setTimeZone(zone.value);
                        setTimeZonePickerOpen(false);
                        setTimeZoneSearch("");
                      }}
                    >
                      <span>{zone.label}</span>
                      <small>{zone.value}</small>
                    </button>
                  ))
                ) : (
                  <p className="time-zone-empty">No matching time zones.</p>
                )}
              </div>
            )}
          </div>
        </div>
        <label className="form-field">
          Default reminder time
          <span className="settings-reminder-control">
            <input
              type="time"
              value={reminderTime}
              onChange={(event) => setReminderTime(event.target.value)}
            />
          </span>
        </label>
      </section>
      <section className="list-card">
        <h2>Notification channels</h2>
        <p>
          Choose which updates may reach you. Email delivery follows your saved
          channel choices.
        </p>
        {state.notificationPrefsStatus === "loading" && (
          <p role="status">Loading notification preferences…</p>
        )}
        {state.notificationPrefsStatus === "error" && (
          <p role="alert">{state.notificationPrefsError}</p>
        )}
        <table className="notification-channel-table">
          <thead>
            <tr>
              <th scope="col">Type</th>
              <th scope="col">In-app</th>
              <th scope="col">Email</th>
            </tr>
          </thead>
          <tbody>
            {[
              "Task assignments",
              "Event changes",
              "Membership changes",
              "Join requests",
              "Join request results",
              "Due reminders",
            ].map((label) => (
              <tr key={label}>
                <th scope="row">{label}</th>
                <td>
                  <input
                    type="checkbox"
                    role="switch"
                    aria-label={`${label} in-app notifications`}
                    checked={notificationPrefs[`${label}:In-app`] ?? false}
                    onChange={(event) =>
                      setNotificationPrefs((current) => ({
                        ...current,
                        [`${label}:In-app`]: event.target.checked,
                      }))
                    }
                  />
                </td>
                <td>
                  <input
                    type="checkbox"
                    role="switch"
                    aria-label={`${label} email notifications`}
                    checked={notificationPrefs[`${label}:Email`] ?? false}
                    onChange={(event) =>
                      setNotificationPrefs((current) => ({
                        ...current,
                        [`${label}:Email`]: event.target.checked,
                      }))
                    }
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {state.notificationPrefsError &&
          state.notificationPrefsStatus !== "error" && (
            <p role="alert">{state.notificationPrefsError}</p>
          )}
      </section>
      <section className="list-card">
        <h2>Muted Spaces</h2>
        <p>
          No in-app or email notifications from muted Spaces. Unmuting won’t
          backfill past notifications. Save your changes below.
        </p>
        {state.activeSpaces.map((s) => (
          <label className="setting-row muted-space-row" key={s.id}>
            <span>{s.name}</span>
            <input
              type="checkbox"
              role="switch"
              aria-label={`Mute ${s.name} notifications`}
              checked={mutedSpaces.includes(s.id)}
              onChange={(event) =>
                setMutedSpaces((current) =>
                  event.target.checked
                    ? [...new Set([...current, s.id])]
                    : current.filter((id) => id !== s.id),
                )
              }
            />
          </label>
        ))}
      </section>
      <section className="list-card">
        <h2>Recently deleted</h2>
        <p>
          Spaces you deleted are restorable for seven days. Previous invitations
          and join links stay invalid after restoration.
        </p>
        {state.deletedSpacesStatus === "loading" && (
          <p role="status">Loading deleted Spaces…</p>
        )}
        {state.deletedSpacesStatus === "error" && (
          <p role="alert">
            Deleted Spaces couldn’t be loaded. Refresh and try again.
          </p>
        )}
        {state.deletedSpacesStatus === "ready" &&
          state.deletedSpaces.length === 0 && (
            <p>No deleted Spaces to restore.</p>
          )}
        {state.deletedSpaces.map((deletedSpace) => (
          <div className="row-between" key={deletedSpace.id}>
            <div>
              <b>{deletedSpace.name}</b>
              <p>
                Deleted {new Date(deletedSpace.deleted_at).toLocaleString()} ·
                Restorable until{" "}
                {new Date(deletedSpace.restore_until).toLocaleString()}
              </p>
            </div>
            <Button
              variant="outline"
              onClick={() => void state.restoreDeletedSpace(deletedSpace.id)}
            >
              Restore
            </Button>
          </div>
        ))}
      </section>
      <section className="settings-save list-card">
        <h2>Save settings</h2>
        <p>
          Save your time zone, reminder time, notification channels, and muted
          Spaces.
        </p>
        <Button
          disabled={
            notificationPrefsStatus === "loading" ||
            notificationPrefsStatus === "saving"
          }
          onClick={() => void state.savePreferences()}
        >
          {notificationPrefsStatus === "saving" ? "Saving…" : "Save settings"}
        </Button>
      </section>
    </>
  );
}
