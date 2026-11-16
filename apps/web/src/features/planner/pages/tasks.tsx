import type { PlannerAppState, PageHelpers } from "./context";
import { Button, Icon, SpaceGlyph } from "../../../shared/ui";

export function TasksPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const {
    setModal,
    space,
    tasks,
    doneOpen,
    setDoneOpen,
    taskLists,
    tasksStatus,
    taskError,
    retryTasks,
  } = state;
  const { pageHeader, taskRow } = helpers;
  const canManage = space.role !== "Viewer";
  return (
    <>
      <div className="space-context-tag">
        <SpaceGlyph space={space} small />
        {space.name}
      </div>
      {pageHeader(
        "To-dos",
        "Shared lists for this Space. Each Task can have one assignee.",
        canManage ? (
          <Button onClick={() => setModal("task")}>
            <Icon name="plus" />
            New Task
          </Button>
        ) : null,
      )}
      {tasksStatus === "loading" && <p role="status">Loading To-dos…</p>}
      {tasksStatus === "error" && (
        <div role="alert">
          <p>{taskError}</p>
          <Button variant="outline" onClick={retryTasks}>
            Retry
          </Button>
        </div>
      )}
      {taskError && tasksStatus !== "error" && <p role="alert">{taskError}</p>}
      <div className="tasks-page-grid">
        {(taskLists[space.id] ?? []).map((list) => {
          const rows = tasks.filter(
            (t) => t.space === space.id && t.list === list,
          );
          const openCount = rows.filter((t) => !t.done).length;
          return (
            <section className="list-card task-list-card" key={list}>
              <div className="list-heading">
                <h2>
                  {list} · {openCount} open
                </h2>
                {canManage &&
                  (() => {
                    const savedList = state.serverTaskLists[space.id]?.find(
                      (item) => item.name === list,
                    );
                    return savedList ? (
                      <span className="list-actions">
                        <button
                          aria-label={`Rename ${list}`}
                          onClick={() => state.editTaskList(savedList.id)}
                        >
                          Rename
                        </button>
                        <button
                          aria-label={`Delete ${list}`}
                          onClick={() => state.removeTaskList(savedList.id)}
                        >
                          Delete
                        </button>
                      </span>
                    ) : null;
                  })()}
              </div>
              {rows.filter((t) => !t.done).map((t) => taskRow(t))}
              {rows.some((t) => t.done) && (
                <>
                  <button
                    className="completed-toggle"
                    onClick={() => setDoneOpen(!doneOpen)}
                  >
                    Completed ({rows.filter((t) => t.done).length})
                  </button>
                  {doneOpen &&
                    rows.filter((t) => t.done).map((t) => taskRow(t))}
                </>
              )}
            </section>
          );
        })}
        {canManage && (
          <section className="new-task-list-card">
            <h2>New list</h2>
            <form
              className="new-task-list-form"
              onSubmit={async (event) => {
                event.preventDefault();
                const form = event.currentTarget;
                const data = new FormData(form);
                if (
                  await state.createTaskList(String(data.get("listName") ?? ""))
                ) {
                  form.reset();
                }
              }}
            >
              <input
                aria-label="List name"
                name="listName"
                placeholder="e.g. Weekend jobs"
                maxLength={80}
                required
              />
              <Button type="submit" disabled={state.savingTask}>
                {state.savingTask ? "Adding…" : "Add"}
              </Button>
            </form>
          </section>
        )}
      </div>
      <div className="task-pagination">
        {state.tasksNextOffset !== null && (
          <Button
            variant="outline"
            disabled={state.loadingMoreTasks}
            onClick={state.loadMoreTasks}
          >
            {state.loadingMoreTasks ? "Loading…" : "Load more Tasks"}
          </Button>
        )}
        {state.listsNextOffset !== null && (
          <Button
            variant="outline"
            disabled={state.loadingMoreLists}
            onClick={state.loadMoreLists}
          >
            {state.loadingMoreLists ? "Loading…" : "Load more To-do lists"}
          </Button>
        )}
      </div>
    </>
  );
}
