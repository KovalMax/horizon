import { useEffect, useState } from "react";
import type { EventItem, TaskItem } from "./domain";
import { listCalendarEvents } from "./eventsApi";
import { listTodos, type ApiTaskList } from "./todosApi";

function dateForOffset(offset: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function useCalendarCollection(
  accessToken: string,
  enabled: boolean,
  overview: boolean,
) {
  const [events, setEvents] = useState<EventItem[]>([]);
  const [eventsStatus, setEventsStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [eventsReload, setEventsReload] = useState(0);
  const [eventError, setEventError] = useState("");
  const [eventsNextOffset, setEventsNextOffset] = useState<number | null>(null);
  const [loadingMoreEvents, setLoadingMoreEvents] = useState(false);
  const [overviewPreview, setOverviewPreview] = useState<{
    event?: EventItem;
  }>();

  useEffect(() => {
    if (!enabled) {
      setEventsStatus("idle");
      return;
    }
    let active = true;
    setEvents([]);
    setEventsNextOffset(null);
    setOverviewPreview(undefined);
    setEventsStatus("loading");
    setEventError("");
    listCalendarEvents(
      accessToken,
      0,
      dateForOffset(0),
      dateForOffset(90),
      undefined,
      overview ? dateForOffset(60) : undefined,
    )
      .then((result) => {
        if (!active) return;
        setEvents(result.items);
        setEventsNextOffset(result.nextOffset);
        if (overview) setOverviewPreview({ event: result.nextEvent });
        setEventsStatus("ready");
      })
      .catch(() => {
        if (!active) return;
        setEventsStatus("error");
        setEventError(
          "Calendar events couldn’t be loaded. Check your connection and retry.",
        );
      });
    return () => {
      active = false;
    };
  }, [accessToken, enabled, eventsReload, overview]);

  return {
    events,
    setEvents,
    eventsStatus,
    setEventsStatus,
    eventsReload,
    setEventsReload,
    eventError,
    setEventError,
    eventsNextOffset,
    setEventsNextOffset,
    loadingMoreEvents,
    setLoadingMoreEvents,
    overviewPreview,
  };
}

export function useTaskCollection(accessToken: string, enabled: boolean) {
  const [tasks, setTasks] = useState<TaskItem[]>([]);
  const [tasksStatus, setTasksStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [tasksReload, setTasksReload] = useState(0);
  const [taskError, setTaskError] = useState("");
  const [listsNextOffset, setListsNextOffset] = useState<number | null>(null);
  const [tasksNextOffset, setTasksNextOffset] = useState<number | null>(null);
  const [loadingMoreLists, setLoadingMoreLists] = useState(false);
  const [loadingMoreTasks, setLoadingMoreTasks] = useState(false);
  const [taskLists, setTaskLists] = useState<Record<string, string[]>>({});
  const [serverTaskLists, setServerTaskLists] = useState<
    Record<string, ApiTaskList[]>
  >({});

  useEffect(() => {
    if (!enabled) {
      setTasksStatus("idle");
      return;
    }
    let active = true;
    setTasks([]);
    setListsNextOffset(null);
    setTasksNextOffset(null);
    setTaskLists({});
    setServerTaskLists({});
    setTasksStatus("loading");
    setTaskError("");
    listTodos(accessToken)
      .then(({ lists, tasks: result, listsNextOffset, tasksNextOffset }) => {
        if (!active) return;
        setTasks(result);
        setListsNextOffset(listsNextOffset);
        setTasksNextOffset(tasksNextOffset);
        setServerTaskLists(
          lists.reduce<Record<string, ApiTaskList[]>>((bySpace, list) => {
            bySpace[list.space_id] = [...(bySpace[list.space_id] ?? []), list];
            return bySpace;
          }, {}),
        );
        setTaskLists(
          lists.reduce<Record<string, string[]>>((bySpace, list) => {
            bySpace[list.space_id] = [
              ...(bySpace[list.space_id] ?? []),
              list.name,
            ];
            return bySpace;
          }, {}),
        );
        setTasksStatus("ready");
      })
      .catch(() => {
        if (!active) return;
        setTasksStatus("error");
        setTaskError(
          "To-dos couldn’t be loaded. Check your connection and retry.",
        );
      });
    return () => {
      active = false;
    };
  }, [accessToken, enabled, tasksReload]);

  return {
    tasks,
    setTasks,
    tasksStatus,
    setTasksStatus,
    tasksReload,
    setTasksReload,
    taskError,
    setTaskError,
    listsNextOffset,
    setListsNextOffset,
    tasksNextOffset,
    setTasksNextOffset,
    loadingMoreLists,
    setLoadingMoreLists,
    loadingMoreTasks,
    setLoadingMoreTasks,
    taskLists,
    setTaskLists,
    serverTaskLists,
    setServerTaskLists,
  };
}
