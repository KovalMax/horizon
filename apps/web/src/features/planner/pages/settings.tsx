import type { PlannerAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

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
  const { pageHeader } = helpers;
  return (
    <>
      {pageHeader("Settings", "Your account and notification preferences.")}
      <section className="list-card">
        <h2>Time and reminders</h2>
        <label className="form-field">
          Time zone (IANA)
          <input
            value={timeZone}
            onChange={(event) => setTimeZone(event.target.value)}
            placeholder="Europe/Berlin"
          />
        </label>
        <label className="form-field">
          Default reminder time
          <input
            type="time"
            value={reminderTime}
            onChange={(event) => setReminderTime(event.target.value)}
          />
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
        {[
          "Task assignments",
          "Event changes",
          "Membership changes",
          "Join requests",
          "Join request results",
          "Due reminders",
        ].map((label) => (
          <div className="setting-row" key={label}>
            <span>{label}</span>
            <label>
              <input
                type="checkbox"
                checked={notificationPrefs[`${label}:In-app`] ?? false}
                onChange={(event) =>
                  setNotificationPrefs((current) => ({
                    ...current,
                    [`${label}:In-app`]: event.target.checked,
                  }))
                }
              />{" "}
              In-app
            </label>
            <label>
              <input
                type="checkbox"
                checked={notificationPrefs[`${label}:Email`] ?? false}
                onChange={(event) =>
                  setNotificationPrefs((current) => ({
                    ...current,
                    [`${label}:Email`]: event.target.checked,
                  }))
                }
              />{" "}
              Email
            </label>
          </div>
        ))}
        {state.notificationPrefsError &&
          state.notificationPrefsStatus !== "error" && (
            <p role="alert">{state.notificationPrefsError}</p>
          )}
        <Button
          disabled={
            notificationPrefsStatus === "loading" ||
            notificationPrefsStatus === "saving"
          }
          onClick={() => void state.savePreferences()}
        >
          {notificationPrefsStatus === "saving"
            ? "Saving…"
            : "Save notification preferences"}
        </Button>
      </section>
      <section className="list-card">
        <h2>Muted Spaces</h2>
        <p>
          In-app notifications from muted Spaces aren’t delivered, and unmuting
          won’t backfill them. Save your changes with the button above.
        </p>
        {state.activeSpaces.map((s) => (
          <label className="setting-row" key={s.id}>
            <span>{s.name}</span>
            <input
              type="checkbox"
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
    </>
  );
}
