import type { PrototypeAppState, PageHelpers } from "./context";
import { Button } from "../../../shared/ui";

export function SettingsPage({
  state,
  helpers,
}: {
  state: PrototypeAppState;
  helpers: PageHelpers;
}) {
  const {
    showToast,
    reminderTime,
    setReminderTime,
    notificationPrefs,
    setNotificationPrefs,
    mutedSpaces,
    setMutedSpaces,
  } = state;
  const { pageHeader } = helpers;
  return (
    <>
      {pageHeader("Settings", "Your account and notification preferences.")}
      <section className="list-card">
        <h2>Account</h2>
        <div className="member-row">
          <span className="avatar">MO</span>
          <span>
            <b>Maya Okafor</b>
            <small>maya@example.com · Verified</small>
          </span>
          <Button
            variant="outline"
            onClick={() => showToast("Account details are simulated.")}
          >
            Edit
          </Button>
        </div>
        <label className="form-field">
          Default reminder time
          <input
            type="time"
            value={reminderTime}
            onChange={(event) => setReminderTime(event.target.value)}
          />
        </label>
        <Button
          onClick={() => showToast("Preferences are active for this session.")}
        >
          Save preferences
        </Button>
      </section>
      <section className="list-card">
        <h2>Notification channels</h2>
        <p>Choose where each type of update should reach you.</p>
        {[
          "Task assignments",
          "Due reminders",
          "Event changes",
          "Membership changes",
          "Join requests",
          "Join request results",
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
      </section>
      <section className="list-card">
        <h2>Muted Spaces</h2>
        <p>Muted Spaces won't send you notifications.</p>
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
        <div className="row-between">
          <div>
            <b>Thursday book club</b>
            <p>Deleted 2 days ago · Restorable for 5 more days</p>
          </div>
          <Button
            variant="outline"
            onClick={() =>
              showToast("Space restored. Old invites and links remain invalid.")
            }
          >
            Restore
          </Button>
        </div>
      </section>
    </>
  );
}
