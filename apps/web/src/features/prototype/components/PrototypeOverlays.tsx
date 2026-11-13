import { dateLabel, spaces } from "../domain";
import type { PrototypeAppState } from "../usePrototypeApp";
import { AppLink, Button, Icon } from "../../../shared/ui";
import { SpaceSwitcher } from "./SpaceSwitcher";

function taskDateAtOffset(offset: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function PrototypeOverlays({
  appState,
}: {
  appState: PrototypeAppState;
}) {
  const {
    mobileMenu,
    setMobileMenu,
    go,
    showToast,
    modal,
    closeModal,
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
  return (
    <>
      <SpaceSwitcher appState={appState} />
      {mobileMenu && (
        <div className="drawer-backdrop" onClick={() => setMobileMenu(false)}>
          <section
            aria-labelledby="more-drawer-title"
            aria-modal="true"
            className="more-drawer"
            onClick={(e) => e.stopPropagation()}
            role="dialog"
          >
            <div className="drawer-heading">
              <h2 id="more-drawer-title">More</h2>
              <button onClick={() => setMobileMenu(false)}>×</button>
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
            className="modal"
            onSubmit={submitModal}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="drawer-heading">
              <h2>
                {modal === "event"
                  ? editingEvent
                    ? "Edit event"
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
              <button type="button" onClick={closeModal}>
                ×
              </button>
            </div>
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
                      modal === "task" ? "What needs doing?" : "Add a title"
                    }
                  />
                </label>
                {modal === "event" && (
                  <div className="modal-grid">
                    <label className="form-field">
                      Date
                      <input
                        name="date"
                        type="date"
                        defaultValue={editingEvent?.startDate}
                      />
                    </label>
                    <label className="form-field">
                      Starts
                      <input
                        name="time"
                        type="time"
                        defaultValue={editingEvent?.time ?? "18:00"}
                      />
                    </label>
                    <label className="form-field">
                      Ends
                      <input
                        name="end"
                        type="time"
                        defaultValue={editingEvent?.end ?? "19:00"}
                      />
                    </label>
                  </div>
                )}
                {modal === "event" && (
                  <>
                    <label className="form-field">
                      Place
                      <input
                        name="place"
                        placeholder="Add a place or address"
                        defaultValue={editingEvent?.place}
                      />
                    </label>
                    <label className="form-field">
                      Repeat
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
                    <label className="setting-row">
                      <span>All day</span>
                      <input
                        name="allDay"
                        type="checkbox"
                        defaultChecked={editingEvent?.allDay}
                      />
                    </label>
                    <label className="form-field">
                      Notes
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
                    <label className="form-field">
                      List
                      <select
                        name="list"
                        defaultValue={
                          appState.editingTask?.list ??
                          appState.taskLists[appState.space.id]?.[0] ??
                          "To-do"
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
                      Due date
                      <input
                        name="dueDate"
                        type="date"
                        defaultValue={
                          appState.editingTask?.day === undefined
                            ? ""
                            : taskDateAtOffset(appState.editingTask.day)
                        }
                      />
                    </label>
                    <div className="modal-grid">
                      <label className="form-field">
                        Time (optional)
                        <input
                          name="taskTime"
                          type="time"
                          defaultValue={appState.editingTask?.time ?? ""}
                        />
                      </label>
                      <label className="form-field">
                        Assigned to
                        <select
                          name={appState.accessToken ? "assignee" : "owner"}
                          defaultValue={appState.editingTask?.assigneeId ?? ""}
                          disabled={
                            Boolean(appState.accessToken) &&
                            appState.membershipStatus !== "ready"
                          }
                        >
                          <option value="">Unassigned</option>
                          {appState.accessToken
                            ? appState.spaceMembers.map((member) => (
                                <option
                                  key={member.user_id}
                                  value={member.user_id}
                                >
                                  {member.email}
                                  {member.role === "owner" ? " · Owner" : ""}
                                </option>
                              ))
                            : ["You", "Jonas", "Ruth", "Sam"].map((name) => (
                                <option key={name}>{name}</option>
                              ))}
                        </select>
                        {appState.accessToken &&
                          appState.membershipStatus !== "ready" && (
                            <small>
                              {appState.membershipStatus === "loading"
                                ? "Loading Space Members…"
                                : "Space Members are unavailable; save this Task unassigned."}
                            </small>
                          )}
                      </label>
                    </div>
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
                            ? "Save event"
                            : modal === "place"
                              ? "Save place"
                              : modal === "list"
                                ? appState.editingList
                                  ? "Save list name"
                                  : "Create list"
                                : "Create Space"}
              </Button>
            </div>
            <small>
              {modal === "space"
                ? "This Space is saved. Events, To-dos, and Places remain session-only prototypes."
                : modal === "event" && appState.accessToken
                  ? "This Calendar event is saved to the selected Space."
                  : (modal === "task" || modal === "list") &&
                      appState.accessToken
                    ? "This To-do content is saved to the selected Space."
                    : modal === "place" && appState.accessToken
                      ? "This Saved place is saved to the selected Space."
                      : "Changes stay in this session and reset when you leave."}
            </small>
          </form>
        </div>
      )}
      {selected && (
        <div className="modal-backdrop" onClick={() => setSelected(null)}>
          <section
            className="modal detail-modal"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="drawer-heading">
              <span className="eyebrow">
                {"space" in selected
                  ? (("spaceName" in selected
                      ? selected.spaceName
                      : undefined) ??
                    spaces.find((s) => s.id === selected.space)?.name ??
                    "Space")
                  : ""}
              </span>
              <button onClick={() => setSelected(null)}>×</button>
            </div>
            <h2>{selected.title}</h2>
            {"time" in selected && (
              <p>
                {selected.time
                  ? `${selected.time}${"end" in selected && selected.end ? `–${selected.end}` : ""} · ${dateLabel(selected.day ?? 0, true)}`
                  : dateLabel(selected.day ?? 0, true)}
              </p>
            )}
            {"place" in selected && selected.place && <p>{selected.place}</p>}
            {"notes" in selected && selected.notes && <p>{selected.notes}</p>}
            {"list" in selected && (
              <p>
                {selected.list}
                {selected.owner ? ` · Assigned to ${selected.owner}` : ""}
                {selected.reminders
                  ? ` · ${selected.reminders} reminder${selected.reminders === 1 ? "" : "s"}`
                  : ""}
              </p>
            )}
            {"zone" in selected &&
              !("list" in selected) &&
              !appState.accessToken && (
                <div className="timezone-note">
                  <b>Time in each timezone</b>
                  <p>
                    London 19:00 · Your time (Berlin) 20:00 · Sam's time (New
                    York) 14:00 · Ana's time (Lisbon) 19:00
                  </p>
                </div>
              )}
            <div className="modal-actions">
              <Button variant="outline" onClick={() => setSelected(null)}>
                Close
              </Button>
              {"list" in selected &&
                (!appState.accessToken ||
                  appState.activeSpaces.some(
                    (space) =>
                      space.id === selected.space && space.role !== "Viewer",
                  )) && (
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
                appState.accessToken &&
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
              {"time" in selected && !("list" in selected) && (
                <Button variant="outline" onClick={editSelectedEvent}>
                  Edit
                </Button>
              )}
              {"time" in selected && !("list" in selected) && (
                <Button variant="outline" onClick={removeSelectedEvent}>
                  Delete
                </Button>
              )}
            </div>
            {eventError && <p role="alert">{eventError}</p>}
            {"list" in selected && taskError && <p role="alert">{taskError}</p>}
            <small>
              {"list" in selected
                ? "Session-only sample data."
                : appState.accessToken
                  ? "Saved Calendar event."
                  : "Session-only sample data."}
            </small>
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
