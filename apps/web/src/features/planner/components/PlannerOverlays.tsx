import { useEffect, useRef, useState } from "react";
import { dateLabel } from "../domain";
import type { PlannerAppState } from "../usePlannerApp";
import { AppLink, Button, Icon } from "../../../shared/ui";
import { SpaceSwitcher } from "./SpaceSwitcher";

const eventTimeValidationMessage = "The end time must be after the start time.";
const dialogFocusableSelector =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
const eventTimeZones = [
  ["Europe/Berlin", "Berlin (CET)"],
  ["Europe/Lisbon", "Lisbon (WET)"],
  ["Europe/London", "London (GMT)"],
  ["America/New_York", "New York (ET)"],
  ["America/Los_Angeles", "Los Angeles (PT)"],
  ["Asia/Kolkata", "Kolkata (IST)"],
  ["Asia/Tokyo", "Tokyo (JST)"],
  ["UTC", "UTC"],
] as const;

function clearCorrectedEventTimeError(
  event: React.ChangeEvent<HTMLInputElement>,
  appState: PlannerAppState,
) {
  if (appState.eventError !== eventTimeValidationMessage) return;
  const form = event.currentTarget.form;
  if (!form) return;
  const values = new FormData(form);
  if (values.get("allDay") === "on") {
    appState.setEventError("");
    return;
  }
  const starts = String(values.get("time") || "");
  const ends = String(values.get("end") || "");
  if (
    starts &&
    ends &&
    new Date(`2000-01-01T${ends}`).getTime() >
      new Date(`2000-01-01T${starts}`).getTime()
  ) {
    appState.setEventError("");
  }
}

function taskDateAtOffset(offset: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function PlannerOverlays({ appState }: { appState: PlannerAppState }) {
  const [taskDueDate, setTaskDueDate] = useState("");
  const [confirmationText, setConfirmationText] = useState("");
  const {
    mobileMenu,
    setMobileMenu,
    go,
    showToast,
    modal,
    closeModal,
    confirmation,
    dismissConfirmation,
    confirmPendingAction,
    submitModal,
    selected,
    setSelected,
    toggleTask,
    toast,
    spacesError,
    creatingSpace,
    editingEvent,
    savingEvent,
    savingTask,
    eventError,
    taskError,
    placeError,
    savingPlace,
    editSelectedEvent,
    removeSelectedEvent,
    editSelectedTask,
    removeSelectedTask,
  } = appState;
  const overlayStateRef = useRef({
    appState,
    closeModal,
    confirmation,
    dismissConfirmation,
    mobileMenu,
    modal,
    selected,
    setMobileMenu,
  });
  overlayStateRef.current = {
    appState,
    closeModal,
    confirmation,
    dismissConfirmation,
    mobileMenu,
    modal,
    selected,
    setMobileMenu,
  };
  const closeMoreMenu = () => {
    setMobileMenu(false);
    window.setTimeout(
      () => document.querySelector<HTMLElement>("[data-more-trigger]")?.focus(),
      0,
    );
  };
  useEffect(() => {
    if (modal !== "task") return;
    setTaskDueDate(
      appState.editingTask?.day === undefined
        ? ""
        : taskDateAtOffset(appState.editingTask.day),
    );
  }, [modal, appState.editingTask]);
  useEffect(() => {
    setConfirmationText("");
  }, [confirmation]);
  useEffect(() => {
    const current = overlayStateRef.current;
    const surface = current.confirmation
      ? document.querySelector<HTMLElement>(
          ".confirmation-modal[role='dialog']",
        )
      : current.selected
        ? document.querySelector<HTMLElement>(".detail-modal[role='dialog']")
        : current.modal
          ? document.querySelector<HTMLElement>(".modal[role='dialog']")
          : current.mobileMenu
            ? document.querySelector<HTMLElement>(".more-drawer[role='dialog']")
            : current.appState.spaceSwitcherOpen
              ? document.querySelector<HTMLElement>(
                  ".space-switcher[role='dialog']",
                )
              : null;
    if (!surface) return;
    const focusable = Array.from(
      surface.querySelectorAll<HTMLElement>(dialogFocusableSelector),
    ).filter((element) => element.getAttribute("aria-hidden") !== "true");
    const initialFocus =
      surface.querySelector<HTMLElement>("[autofocus]") ?? focusable[0];
    initialFocus?.focus();

    const trapAndDismiss = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        const latest = overlayStateRef.current;
        if (latest.confirmation) latest.dismissConfirmation();
        else if (latest.selected) latest.appState.setSelected(null);
        else if (latest.modal) latest.closeModal();
        else if (latest.mobileMenu) {
          latest.setMobileMenu(false);
          window.setTimeout(
            () =>
              document
                .querySelector<HTMLElement>("[data-more-trigger]")
                ?.focus(),
            0,
          );
        } else if (latest.appState.spaceSwitcherOpen) {
          latest.appState.setSpaceSwitcherOpen(false);
          window.setTimeout(
            () =>
              document
                .querySelector<HTMLElement>("[data-space-switcher-trigger]")
                ?.focus(),
            0,
          );
        }
        return;
      }
      if (event.key !== "Tab" || focusable.length === 0) return;
      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    surface.addEventListener("keydown", trapAndDismiss);
    return () => surface.removeEventListener("keydown", trapAndDismiss);
  }, [
    appState.spaceSwitcherOpen,
    confirmation?.title,
    mobileMenu,
    modal,
    selected?.id,
  ]);
  return (
    <>
      <SpaceSwitcher appState={appState} />
      {mobileMenu && (
        <div className="drawer-backdrop" onClick={closeMoreMenu}>
          <section
            aria-labelledby="more-drawer-title"
            aria-modal="true"
            className="more-drawer"
            onClick={(e) => e.stopPropagation()}
            role="dialog"
          >
            <div className="drawer-heading">
              <h2 id="more-drawer-title">More</h2>
              <button
                type="button"
                aria-label="Close More menu"
                onClick={closeMoreMenu}
              >
                ×
              </button>
            </div>
            <nav aria-label="More" className="more-links">
              <AppLink
                to={`/spaces/${appState.space.id}/members`}
                onNavigate={go}
              >
                <Icon name="users" /> Members & sharing
              </AppLink>
              <AppLink
                to={`/spaces/${appState.space.id}/settings`}
                onNavigate={go}
              >
                <Icon name="settings" /> Space settings
              </AppLink>
              <AppLink to="/settings" onNavigate={go}>
                <Icon name="settings" /> Notification settings
              </AppLink>
            </nav>
          </section>
        </div>
      )}
      {modal && (
        <div className="modal-backdrop" onClick={closeModal}>
          <form
            role="dialog"
            aria-modal="true"
            aria-labelledby="create-edit-dialog-title"
            className={`modal ${modal}-modal`}
            onSubmit={submitModal}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="drawer-heading">
              <h2 id="create-edit-dialog-title">
                {modal === "event"
                  ? editingEvent
                    ? appState.eventEditScope === "occurrence"
                      ? "Edit occurrence"
                      : "Edit event"
                    : "New event"
                  : modal === "task"
                    ? appState.editingTask
                      ? "Edit Task"
                      : "New Task"
                    : modal === "place"
                      ? "Save a place"
                      : modal === "list"
                        ? appState.editingList
                          ? "Rename To-do list"
                          : "New list"
                        : "New shared Space"}
              </h2>
              <button
                type="button"
                aria-label="Close dialog"
                onClick={closeModal}
              >
                ×
              </button>
            </div>
            {modal === "task" && (
              <p className="modal-context">In {appState.space.name}</p>
            )}
            {modal === "space" ? (
              <>
                <p>
                  A shared Space is saved to your account and visible to its
                  members.
                </p>
                <label className="form-field">
                  Space name
                  <input
                    name="title"
                    required
                    placeholder="e.g. Weekend plans"
                  />
                </label>
                {spacesError && <p role="alert">{spacesError}</p>}
              </>
            ) : modal === "list" ? (
              <label className="form-field">
                List name
                <input
                  name="title"
                  required
                  autoFocus
                  defaultValue={appState.editingList?.name}
                  placeholder="e.g. Weekend prep"
                />
              </label>
            ) : (
              <>
                {modal === "event" && (
                  <label className="form-field">
                    Space
                    <select
                      name="space"
                      defaultValue={editingEvent?.space ?? appState.space.id}
                      disabled={Boolean(editingEvent)}
                    >
                      {appState.activeSpaces
                        .filter((item) => item.role !== "Viewer")
                        .map((item) => (
                          <option key={item.id} value={item.id}>
                            {item.name}
                          </option>
                        ))}
                    </select>
                  </label>
                )}
                <label className="form-field">
                  {modal === "place" ? "Place name" : "Title"}
                  <input
                    name="title"
                    required
                    autoFocus
                    defaultValue={
                      modal === "task"
                        ? appState.editingTask?.title
                        : editingEvent?.title
                    }
                    placeholder={
                      modal === "task"
                        ? "What needs doing?"
                        : modal === "event"
                          ? "e.g. Parents' evening"
                          : "Add a title"
                    }
                  />
                </label>
                {modal === "event" && (
                  <label className="setting-row event-all-day-row">
                    <span>All-day event</span>
                    <input
                      name="allDay"
                      type="checkbox"
                      role="switch"
                      aria-label="All-day event"
                      defaultChecked={editingEvent?.allDay}
                      onChange={(event) =>
                        clearCorrectedEventTimeError(event, appState)
                      }
                    />
                  </label>
                )}
                {modal === "event" && (
                  <div className="modal-grid event-date-grid">
                    <label className="form-field">
                      Date
                      <input
                        name="date"
                        type="date"
                        defaultValue={
                          editingEvent?.startDate ??
                          taskDateAtOffset(appState.calendarDay)
                        }
                      />
                    </label>
                    <label className="form-field">
                      Starts
                      <input
                        name="time"
                        type="time"
                        defaultValue={editingEvent?.time ?? "18:00"}
                        onChange={(event) =>
                          clearCorrectedEventTimeError(event, appState)
                        }
                      />
                    </label>
                    <label className="form-field">
                      Ends
                      <input
                        name="end"
                        type="time"
                        defaultValue={editingEvent?.end ?? "19:00"}
                        onChange={(event) =>
                          clearCorrectedEventTimeError(event, appState)
                        }
                      />
                    </label>
                  </div>
                )}
                {modal === "event" && (
                  <>
                    <label className="form-field">
                      Event timezone
                      <select
                        name="timeZone"
                        defaultValue={
                          editingEvent?.timeZone ?? appState.timeZone
                        }
                      >
                        {eventTimeZones.map(([value, label]) => (
                          <option key={value} value={value}>
                            {label}
                          </option>
                        ))}
                        {!eventTimeZones.some(
                          ([value]) =>
                            value ===
                            (editingEvent?.timeZone ?? appState.timeZone),
                        ) && (
                          <option value={appState.timeZone}>
                            {appState.timeZone}
                          </option>
                        )}
                      </select>
                    </label>
                    {appState.eventEditScope !== "occurrence" && (
                      <label className="form-field">
                        Repeats
                        <select
                          name="repeat"
                          defaultValue={editingEvent?.repeat ?? ""}
                        >
                          <option value="">Does not repeat</option>
                          <option>Every day</option>
                          <option>Every week</option>
                          <option>Every month</option>
                        </select>
                      </label>
                    )}
                    {appState.eventEditScope !== "occurrence" && (
                      <details className="recurrence-end-options">
                        <summary>Set an end date (optional)</summary>
                        <label className="form-field">
                          Repeat through
                          <input
                            name="recurrenceEndsOn"
                            type="date"
                            defaultValue={editingEvent?.recurrenceEndsOn}
                          />
                          <small>Leave blank to repeat indefinitely.</small>
                        </label>
                      </details>
                    )}
                    <label className="form-field">
                      Location (optional)
                      <input
                        name="place"
                        placeholder="Address or place name"
                        defaultValue={editingEvent?.place}
                      />
                    </label>
                    <label className="form-field">
                      Notes (optional)
                      <textarea
                        name="notes"
                        rows={3}
                        defaultValue={editingEvent?.notes}
                      />
                    </label>
                  </>
                )}
                {modal === "task" && (
                  <>
                    <div className="modal-grid">
                      <label className="form-field">
                        List
                        <select
                          name="list"
                          defaultValue={
                            appState.editingTask?.list ??
                            (appState.taskListDefault ||
                              appState.taskLists[appState.space.id]?.[0] ||
                              "To-do")
                          }
                        >
                          {(
                            appState.taskLists[appState.space.id] ?? ["To-do"]
                          ).map((list) => (
                            <option key={list}>{list}</option>
                          ))}
                        </select>
                      </label>
                      <label className="form-field">
                        Assignee (optional, one person)
                        <select
                          name="assignee"
                          defaultValue={appState.editingTask?.assigneeId ?? ""}
                          disabled={appState.membershipStatus !== "ready"}
                        >
                          <option value="">Unassigned</option>
                          {appState.spaceMembers.map((member) => (
                            <option key={member.user_id} value={member.user_id}>
                              {member.email}
                              {member.role === "owner" ? " · Owner" : ""}
                            </option>
                          ))}
                        </select>
                        {appState.membershipStatus !== "ready" && (
                          <small>
                            {appState.membershipStatus === "loading"
                              ? "Loading Space Members…"
                              : "Space Members are unavailable; save this Task unassigned."}
                          </small>
                        )}
                      </label>
                    </div>
                    <div className="modal-grid">
                      <label className="form-field">
                        Due date (optional)
                        <input
                          name="dueDate"
                          type="date"
                          defaultValue={
                            appState.editingTask?.day === undefined
                              ? ""
                              : taskDateAtOffset(appState.editingTask.day)
                          }
                          onChange={(event) =>
                            setTaskDueDate(event.currentTarget.value)
                          }
                        />
                      </label>
                      <label className="form-field">
                        Due time (optional)
                        <input
                          name="taskTime"
                          type="time"
                          disabled={!taskDueDate}
                          defaultValue={appState.editingTask?.time ?? ""}
                        />
                      </label>
                    </div>
                    <section className="task-reminders">
                      <h3 id="task-reminders-heading">Reminders</h3>
                      {taskDueDate ? (
                        <fieldset
                          className="form-field"
                          aria-labelledby="task-reminders-heading"
                        >
                          {[
                            { value: 1440, label: "1 day before" },
                            { value: 60, label: "1 hour before" },
                          ].map((offset) => (
                            <label className="setting-row" key={offset.value}>
                              <span>{offset.label}</span>
                              <input
                                type="checkbox"
                                name="reminderOffset"
                                value={offset.value}
                                defaultChecked={(
                                  appState.editingTask
                                    ?.reminderOffsetsMinutes ?? [1440, 60]
                                ).includes(offset.value)}
                              />
                            </label>
                          ))}
                        </fieldset>
                      ) : (
                        <p>Add a due date to set reminders.</p>
                      )}
                    </section>
                  </>
                )}
                {modal === "place" && (
                  <>
                    <label className="form-field">
                      Address
                      <input
                        name="address"
                        placeholder="Street, city"
                        required
                      />
                    </label>
                    <label className="form-field">
                      Category
                      <select name="category">
                        <option>Food & drink</option>
                        <option>Groceries</option>
                        <option>Health</option>
                        <option>Services</option>
                      </select>
                    </label>
                    <label className="form-field">
                      Note (optional)
                      <textarea
                        name="note"
                        rows={2}
                        placeholder="A useful detail for everyone"
                      />
                    </label>
                  </>
                )}
              </>
            )}
            {modal === "event" && eventError && (
              <p role="alert">{eventError}</p>
            )}
            {(modal === "task" || modal === "list") && taskError && (
              <p role="alert">{taskError}</p>
            )}
            {modal === "place" && placeError && (
              <p role="alert">{placeError}</p>
            )}
            <div className="modal-actions">
              <Button
                variant="outline"
                onClick={closeModal}
                disabled={
                  creatingSpace || savingEvent || savingTask || savingPlace
                }
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={
                  creatingSpace || savingEvent || savingTask || savingPlace
                }
              >
                {savingEvent
                  ? "Saving…"
                  : savingTask
                    ? "Saving…"
                    : savingPlace
                      ? "Saving…"
                      : creatingSpace
                        ? "Creating…"
                        : modal === "task"
                          ? appState.editingTask
                            ? "Save Task changes"
                            : "Add Task"
                          : modal === "event"
                            ? appState.editingEvent
                              ? "Save event"
                              : "Add event"
                            : modal === "place"
                              ? "Save place"
                              : modal === "list"
                                ? appState.editingList
                                  ? "Save list name"
                                  : "Create list"
                                : "Create Space"}
              </Button>
            </div>
            {modal !== "event" && (
              <small>
                {modal === "space"
                  ? "Your new Space is ready for planning."
                  : modal === "task" || modal === "list"
                    ? "This To-do content is saved to the selected Space."
                    : modal === "place"
                      ? "This Saved place is saved to the selected Space."
                      : "Your changes are saved to this Space."}
              </small>
            )}
          </form>
        </div>
      )}
      {selected && (
        <div className="modal-backdrop" onClick={() => setSelected(null)}>
          <section
            className="modal detail-modal"
            inert={Boolean(confirmation)}
            role="dialog"
            aria-modal="true"
            aria-labelledby="selected-item-title"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="drawer-heading">
              <span className="eyebrow">
                {"space" in selected
                  ? (("spaceName" in selected
                      ? selected.spaceName
                      : undefined) ??
                    selected.spaceName ??
                    "Space")
                  : ""}
              </span>
              <button
                type="button"
                aria-label="Close details"
                onClick={() => setSelected(null)}
              >
                ×
              </button>
            </div>
            <h2 id="selected-item-title">{selected.title}</h2>
            {"time" in selected && (
              <p>
                {"list" in selected && selected.day === undefined
                  ? "No due date"
                  : selected.time
                    ? `${selected.time}${"end" in selected && selected.end ? `–${selected.end}` : ""} · ${dateLabel(selected.day ?? 0, true)}`
                    : dateLabel(selected.day ?? 0, true)}
              </p>
            )}
            {"place" in selected && selected.place && <p>{selected.place}</p>}
            {"notes" in selected && selected.notes && <p>{selected.notes}</p>}
            {"list" in selected && (
              <p>
                {selected.list}
                {selected.assigneeEmail
                  ? ` · Assigned to ${selected.assigneeEmail}`
                  : ""}
                {selected.reminders
                  ? ` · ${selected.reminders} reminder${selected.reminders === 1 ? "" : "s"}`
                  : ""}
              </p>
            )}
            {"list" in selected && (
              <form
                onSubmit={(event) => {
                  event.preventDefault();
                  const offsets = Array.from(
                    new FormData(event.currentTarget).getAll("myReminder"),
                    Number,
                  );
                  void appState.saveMyTaskReminders(
                    String(selected.id),
                    offsets,
                  );
                }}
              >
                <fieldset className="form-field">
                  <legend>Your reminders only</legend>
                  {[
                    { value: 1440, label: "1 day before" },
                    { value: 60, label: "1 hour before" },
                  ].map((offset) => (
                    <label className="setting-row" key={offset.value}>
                      <span>{offset.label}</span>
                      <input
                        type="checkbox"
                        name="myReminder"
                        value={offset.value}
                        defaultChecked={(
                          selected.myReminderOffsetsMinutes ??
                          selected.reminderOffsetsMinutes ?? [1440, 60]
                        ).includes(offset.value)}
                      />
                    </label>
                  ))}
                </fieldset>
                <button className="button primary" type="submit">
                  Save my reminders
                </button>
              </form>
            )}
            <div className="modal-actions">
              <Button variant="outline" onClick={() => setSelected(null)}>
                Close
              </Button>
              {"list" in selected &&
                appState.activeSpaces.some(
                  (space) =>
                    space.id === selected.space && space.role !== "Viewer",
                ) && (
                  <Button
                    onClick={() => {
                      toggleTask(selected.id);
                      setSelected(null);
                    }}
                  >
                    {selected.done ? "Reopen Task" : "Mark complete"}
                  </Button>
                )}
              {"list" in selected &&
                selected.space === appState.space.id &&
                appState.space.role !== "Viewer" && (
                  <>
                    <Button variant="outline" onClick={editSelectedTask}>
                      Edit Task
                    </Button>
                    <Button variant="outline" onClick={removeSelectedTask}>
                      Delete Task
                    </Button>
                  </>
                )}
              {"time" in selected &&
              !("list" in selected) &&
              appState.activeSpaces.some(
                (space) =>
                  space.id === selected.space && space.role !== "Viewer",
              ) &&
              selected.repeat &&
              selected.occurrenceDate ? (
                <>
                  <Button
                    variant="outline"
                    onClick={() => editSelectedEvent("occurrence")}
                  >
                    Edit this occurrence
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => editSelectedEvent("series")}
                  >
                    Edit series
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => removeSelectedEvent("occurrence")}
                    disabled={appState.removingEvent}
                  >
                    Cancel this occurrence
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => removeSelectedEvent("series")}
                    disabled={appState.removingEvent}
                  >
                    Delete series
                  </Button>
                </>
              ) : "time" in selected && !("list" in selected) ? (
                appState.activeSpaces.some(
                  (space) =>
                    space.id === selected.space && space.role !== "Viewer",
                ) ? (
                  <>
                    <Button
                      variant="outline"
                      onClick={() => editSelectedEvent("series")}
                    >
                      Edit event
                    </Button>
                    <Button
                      variant="outline"
                      onClick={() => removeSelectedEvent("series")}
                      disabled={appState.removingEvent}
                    >
                      Delete event
                    </Button>
                  </>
                ) : null
              ) : null}
            </div>
            {eventError && <p role="alert">{eventError}</p>}
            {"list" in selected && taskError && <p role="alert">{taskError}</p>}
            <small>
              {"list" in selected ? "Saved Task." : "Saved Calendar event."}
            </small>
          </section>
        </div>
      )}
      {confirmation && (
        <div
          className="modal-backdrop confirmation-backdrop"
          onClick={dismissConfirmation}
        >
          <section
            className="modal confirmation-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirmation-title"
            aria-describedby="confirmation-description"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="drawer-heading">
              <h2 id="confirmation-title">{confirmation.title}</h2>
              <button
                type="button"
                aria-label="Close confirmation"
                onClick={dismissConfirmation}
              >
                ×
              </button>
            </div>
            <p id="confirmation-description">{confirmation.description}</p>
            {confirmation.requiredText && (
              <label className="form-field">
                Type <b>{confirmation.requiredText}</b> to confirm
                <input
                  autoFocus
                  value={confirmationText}
                  onChange={(event) => setConfirmationText(event.target.value)}
                />
              </label>
            )}
            <div className="modal-actions">
              <Button variant="outline" onClick={dismissConfirmation}>
                Cancel
              </Button>
              <Button
                className="confirmation-submit"
                onClick={confirmPendingAction}
                disabled={
                  confirmation.requiredText !== undefined &&
                  confirmationText !== confirmation.requiredText
                }
              >
                {confirmation.confirmLabel}
              </Button>
            </div>
          </section>
        </div>
      )}
      {toast && (
        <div className="toast" role="status">
          {toast}
        </div>
      )}
    </>
  );
}
