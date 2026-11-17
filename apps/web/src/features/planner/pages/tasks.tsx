import { useState } from "react";
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
    taskLists,
    tasksStatus,
    taskError,
    retryTasks,
  } = state;
  const { pageHeader, taskRow } = helpers;
  const canManage = space.role !== "Viewer";
  const [expandedCompletedLists, setExpandedCompletedLists] = useState<
    Set<string>
  >(() => new Set());
  const tasksReady = tasksStatus === "ready";
  return (
    <>
      <div className="space-context-tag">
        <SpaceGlyph space={space} small />
        {space.name}
      </div>
      {pageHeader(
        "To-dos",
        "Shared lists for this Space. Each Task can have one assignee.",
        canManage && tasksReady ? (
          <Button onClick={() => setModal("task")}>
            <Icon name="plus" />
            New Task
          </Button>
        ) : null,
      )}
      {!canManage && (
        <p className="read-only-notice">
          You can view To-dos in this Space, but only Editors and the Owner can
          change them.
        </p>
      )}
      {tasksStatus === "loading" && <p role="status">Loading To-dos…</p>}
      {tasksStatus === "error" && (
        <section
          className="tasks-error-card"
          aria-labelledby="tasks-error-title"
        >
          <div>
            <h2 id="tasks-error-title">To-dos couldn’t load</h2>
            <p role="alert">{taskError}</p>
          </div>
          <Button onClick={retryTasks}>Try again</Button>
        </section>
      )}
      {taskError && tasksStatus !== "error" && <p role="alert">{taskError}</p>}
      <div className="tasks-page-grid" hidden={!tasksReady}>
        {(taskLists[space.id] ?? []).map((list) => {
          const rows = tasks.filter(
            (t) => t.space === space.id && t.list === list,
          );
          const openCount = rows.filter((t) => !t.done).length;
          const completedOpen = expandedCompletedLists.has(list);
          return (
            <section className="list-card task-list-card" key={list}>
              <div className="list-heading">
                <h2>
                  {list} · {openCount} open
                </h2>
                <div className="list-heading-actions">
                  {canManage && (
                    <button
                      className="task-list-add"
                      aria-label={`Add Task to ${list}`}
                      title={`Add Task to ${list}`}
                      onClick={() => {
                        state.setTaskListDefault(list);
                        setModal("task");
                      }}
                    >
                      <Icon name="plus" size={22} />
                    </button>
                  )}
                  {canManage &&
                    (() => {
                      const savedList = state.serverTaskLists[space.id]?.find(
                        (item) => item.name === list,
                      );
                      return savedList ? (
                        <span className="list-actions">
                          <button
                            className="task-list-icon-action"
                            aria-label={`Rename ${list}`}
                            title={`Rename ${list}`}
                            onClick={() => state.editTaskList(savedList.id)}
                          >
                            <Icon name="edit" size={17} />
                          </button>
                          <button
                            className="task-list-icon-action danger"
                            aria-label={`Delete ${list}`}
                            title={`Delete ${list}`}
                            onClick={() => state.removeTaskList(savedList.id)}
                          >
                            <Icon name="trash" size={17} />
                          </button>
                        </span>
                      ) : null;
                    })()}
                </div>
              </div>
              {rows.filter((t) => !t.done).map((t) => taskRow(t))}
              {rows.some((t) => t.done) && (
                <>
                  <button
                    className="completed-toggle"
                    aria-expanded={completedOpen}
                    onClick={() =>
                      setExpandedCompletedLists((current) => {
                        const next = new Set(current);
                        if (next.has(list)) next.delete(list);
                        else next.add(list);
                        return next;
                      })
                    }
                  >
                    <Icon
                      name={completedOpen ? "chevron-up" : "chevron-down"}
                      size={17}
                    />
                    Completed ({rows.filter((t) => t.done).length})
                  </button>
                  {completedOpen &&
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
      <div className="task-pagination" hidden={!tasksReady}>
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
