import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import {
  type Space,
  type EventItem,
  type PlaceItem,
  type TaskItem,
} from "./domain";
import { checkServiceReadiness } from "../../shared/api/health";
import {
  createSharedSpace,
  listDeletedSpaces,
  listSpaces,
  SpacesApiError,
  type DeletedSpace,
} from "./spacesApi";
import {
  CalendarEventsApiError,
  createCalendarEvent,
  deleteCalendarEvent,
  updateCalendarOccurrence,
  cancelCalendarOccurrence,
  listCalendarEvents,
  updateCalendarEvent,
  type CalendarEventInput,
} from "./eventsApi";
import {
  createTodoList,
  createTodoTask,
  deleteTodoList,
  deleteTodoTask,
  listTodos,
  TodosApiError,
  updateTodoCompletion,
  updateTodoList,
  updateTodoTask,
  updateMyTaskReminders,
  type ApiTaskList,
} from "./todosApi";
import {
  createSavedPlace,
  deleteSavedPlace,
  listSavedPlaces,
  PlacesApiError,
} from "./placesApi";
import {
  useCalendarCollection,
  useSavedPlaceCollection,
  useTaskCollection,
} from "./usePlannerCollections";
import { useNotifications } from "./useNotifications";
import { useSpaceMembership } from "./useSpaceMembership";

type ModalKind = "event" | "task" | "place" | "space" | "list" | "";

type ConfirmationRequest = {
  title: string;
  description: string;
  confirmLabel: string;
  requiredText?: string;
  onConfirm: () => void | Promise<unknown>;
  returnFocus: HTMLElement | null;
};

const taskPermissionError =
  "Only an Editor or Owner can change To-dos in this Space.";

function dateForOffset(offset: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function localDateTimeToRfc3339(
  date: string,
  time: string,
  timeZone: string,
): string {
  const [year, month, day] = date.split("-").map(Number);
  const [hour, minute] = time.split(":").map(Number);
  const target = Date.UTC(year, month - 1, day, hour, minute);
  const formatter = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  });
  let instant = target;
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const parts = Object.fromEntries(
      formatter
        .formatToParts(new Date(instant))
        .filter((part) => part.type !== "literal")
        .map((part) => [part.type, Number(part.value)]),
    );
    const represented = Date.UTC(
      parts.year,
      parts.month - 1,
      parts.day,
      parts.hour,
      parts.minute,
      parts.second,
    );
    instant += target - represented;
  }
  const resolved = formatter.formatToParts(new Date(instant));
  const actual = Object.fromEntries(
    resolved
      .filter((part) => part.type !== "literal")
      .map((part) => [part.type, Number(part.value)]),
  );
  if (
    actual.year !== year ||
    actual.month !== month ||
    actual.day !== day ||
    actual.hour !== hour ||
    actual.minute !== minute
  ) {
    throw new Error("The selected time does not exist in this timezone");
  }
  return new Date(instant).toISOString();
}

export function usePlannerApp(accessToken: string) {
  const [ready, setReady] = useState<"checking" | "available" | "unavailable">(
    "checking",
  );
  const [isOffline, setIsOffline] = useState(() => !navigator.onLine);
  const [path, setPath] = useState(window.location.pathname);
  const [serverSpaces, setServerSpaces] = useState<Space[]>([]);
  const [spacesStatus, setSpacesStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [spacesReload, setSpacesReload] = useState(0);
  const [spacesError, setSpacesError] = useState("");
  const [deletedSpaces, setDeletedSpaces] = useState<DeletedSpace[]>([]);
  const [deletedSpacesStatus, setDeletedSpacesStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [creatingSpace, setCreatingSpace] = useState(false);
  const [savingEvent, setSavingEvent] = useState(false);
  const [editingEvent, setEditingEvent] = useState<EventItem | null>(null);
  const [eventEditScope, setEventEditScope] = useState<"occurrence" | "series">(
    "series",
  );
  const [savingTask, setSavingTask] = useState(false);
  const [editingTask, setEditingTask] = useState<TaskItem | null>(null);
  const [editingList, setEditingList] = useState<ApiTaskList | null>(null);
  const [savingPlace, setSavingPlace] = useState(false);
  const [removingPlaceId, setRemovingPlaceId] = useState<string | null>(null);
  const [hidden, setHidden] = useState<string[]>([]);
  const [filter, setFilter] = useState("all");
  const [taskListDefault, setTaskListDefault] = useState("");
  const [modal, setModalState] = useState<ModalKind>("");
  const [confirmation, setConfirmation] = useState<ConfirmationRequest | null>(
    null,
  );
  const requestConfirmation = (
    request: Omit<ConfirmationRequest, "returnFocus">,
  ) => {
    setConfirmation({
      ...request,
      returnFocus:
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null,
    });
  };
  const restoreConfirmationFocus = (target: HTMLElement | null) => {
    window.setTimeout(() => {
      if (target?.isConnected) {
        target.focus();
        return;
      }
      document.querySelector<HTMLElement>(".main-content h1")?.focus();
    }, 0);
  };
  const dismissConfirmation = () => {
    const target = confirmation?.returnFocus ?? null;
    setConfirmation(null);
    restoreConfirmationFocus(target);
  };
  const confirmPendingAction = () => {
    if (!confirmation) return;
    const pending = confirmation;
    setConfirmation(null);
    void pending.onConfirm();
    restoreConfirmationFocus(pending.returnFocus);
  };
  const [selected, setSelectedState] = useState<EventItem | TaskItem | null>(
    null,
  );
  const focusReturnTarget = useRef<HTMLElement | null>(null);
  const restoreFocus = () => {
    const target = focusReturnTarget.current;
    focusReturnTarget.current = null;
    if (target)
      window.setTimeout(() => target.isConnected && target.focus(), 0);
  };
  const setModal = (next: ModalKind) => {
    if (
      next &&
      !selected &&
      !focusReturnTarget.current &&
      document.activeElement instanceof HTMLElement
    ) {
      focusReturnTarget.current = document.activeElement;
    }
    setModalState(next);
  };
  const setSelected = (next: EventItem | TaskItem | null) => {
    if (
      next &&
      !modal &&
      !focusReturnTarget.current &&
      document.activeElement instanceof HTMLElement
    ) {
      focusReturnTarget.current = document.activeElement;
    }
    setSelectedState(next);
    if (!next && !modal) restoreFocus();
  };
  const [placeFilter, setPlaceFilter] = useState("All");
  const [calendarDay, setCalendarDay] = useState(0);
  const [toast, setToast] = useState("");
  const [mobileMenu, setMobileMenu] = useState(false);
  const [spaceSwitcherOpen, setSpaceSwitcherOpen] = useState(false);
  const currentSpaceId =
    path.match(/^\/spaces\/([^/]+)/)?.[1] ?? serverSpaces[0]?.id ?? "";
  const routeSection = path.split("/")[3] ?? "calendar";
  const showOverview = path === "/";
  const showCalendar =
    showOverview ||
    (path.startsWith("/spaces/") && routeSection === "calendar");
  const showTasks =
    showOverview || (path.startsWith("/spaces/") && routeSection === "tasks");
  const showPlaces = path.startsWith("/spaces/") && routeSection === "places";
  const showToast = (message: string) => setToast(message);
  const {
    reminderTime,
    setReminderTime,
    timeZone,
    setTimeZone,
    notificationPrefs,
    setNotificationPrefs,
    notificationPrefsStatus,
    notificationPrefsError,
    mutedSpaces,
    setMutedSpaces,
    setSpaceMuted,
    unread,
    notifications,
    notificationsStatus,
    notificationsError,
    notificationActionError,
    notificationsNextOffset,
    loadingMoreNotifications,
    markNotificationRead,
    markAllNotificationsRead,
    loadMoreNotifications,
    savePreferences,
    retryNotifications,
  } = useNotifications(accessToken, path, showToast);
  const {
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
  } = useCalendarCollection(accessToken, showCalendar);
  const {
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
  } = useTaskCollection(accessToken, showTasks);
  const {
    places,
    setPlaces,
    placesStatus,
    setPlacesStatus,
    placesReload,
    setPlacesReload,
    placeError,
    setPlaceError,
    placesNextOffset,
    setPlacesNextOffset,
    loadingMorePlaces,
    setLoadingMorePlaces,
  } = useSavedPlaceCollection(accessToken, showPlaces);

  useEffect(() => {
    const controller = new AbortController();
    checkServiceReadiness(controller.signal)
      .then((result) => {
        if (!controller.signal.aborted) setReady(result);
      })
      .catch(() => {
        if (!controller.signal.aborted) setReady("unavailable");
      });
    return () => controller.abort();
  }, []);

  useEffect(() => {
    const onOffline = () => setIsOffline(true);
    const onOnline = () => setIsOffline(false);
    window.addEventListener("offline", onOffline);
    window.addEventListener("online", onOnline);
    return () => {
      window.removeEventListener("offline", onOffline);
      window.removeEventListener("online", onOnline);
    };
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    setSpacesStatus("loading");
    setSpacesError("");
    listSpaces(accessToken)
      .then((result) => {
        if (controller.signal.aborted) return;
        setServerSpaces(result);
        setSpacesStatus("ready");
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setSpacesStatus("error");
        setSpacesError(
          "Spaces couldn’t be loaded. Check your connection and try again.",
        );
      });
    return () => controller.abort();
  }, [accessToken, spacesReload]);

  useEffect(() => {
    let active = true;
    setDeletedSpacesStatus("loading");
    listDeletedSpaces(accessToken)
      .then((result) => {
        if (!active) return;
        setDeletedSpaces(result);
        setDeletedSpacesStatus("ready");
      })
      .catch(() => {
        if (!active) return;
        setDeletedSpacesStatus("error");
      });
    return () => {
      active = false;
    };
  }, [accessToken, spacesReload]);

  useEffect(() => {
    const onPopState = () => setPath(window.location.pathname);
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timeoutId = window.setTimeout(() => setToast(""), 2800);
    return () => window.clearTimeout(timeoutId);
  }, [toast]);

  const go = (to: string) => {
    window.history.pushState({}, "", to);
    setPath(window.location.pathname);
    setMobileMenu(false);
    setSpaceSwitcherOpen(false);
    window.scrollTo(0, 0);
  };
  const membershipState = useSpaceMembership({
    accessToken,
    currentSpaceId,
    serverSpaces,
    setServerSpaces,
    setSpacesReload,
    setSpacesError,
    setDeletedSpaces,
    go,
    showToast,
  });

  const selectedSpaceId = path.match(/^\/spaces\/([^/]+)/)?.[1] ?? "";
  const activeSpaces = serverSpaces;
  const baseSpace = activeSpaces.find((item) => item.id === selectedSpaceId) ??
    activeSpaces[0] ?? {
      id: "",
      name: "",
      role: "Viewer" as const,
      members: 0,
      tone: "home",
      icon: "home",
    };
  const space = {
    ...baseSpace,
    name: baseSpace.name,
  };
  const isSpacePage = path.startsWith("/spaces/");
  const belongsToCurrentSpaces = (spaceId: string) =>
    activeSpaces.some((item) => item.id === spaceId);
  const filteredEvents = events.filter(
    (event) =>
      belongsToCurrentSpaces(event.space) &&
      !hidden.includes(event.space) &&
      (!isSpacePage || event.space === space.id) &&
      filter !== "tasks",
  );
  const filteredTasks = tasks.filter(
    (task) =>
      belongsToCurrentSpaces(task.space) &&
      !task.done &&
      !hidden.includes(task.space) &&
      (!isSpacePage || task.space === space.id) &&
      filter !== "events",
  );
  const days = useMemo(
    () => Array.from({ length: 14 }, (_, index) => index),
    [],
  );

  const toggleTask = (id: TaskItem["id"]) => {
    const task = tasks.find((item) => item.id === id);
    if (!task) return;
    if (
      !activeSpaces.some(
        (item) => item.id === task.space && item.role !== "Viewer",
      )
    ) {
      setTaskError(taskPermissionError);
      return;
    }
    setTaskError("");
    updateTodoCompletion(accessToken, id, !task.done)
      .then((updated) =>
        setTasks((all) => all.map((item) => (item.id === id ? updated : item))),
      )
      .catch((error) =>
        setTaskError(
          error instanceof TodosApiError && error.status === 403
            ? taskPermissionError
            : "The Task couldn’t be updated. Check your connection and retry.",
        ),
      );
  };
  const goSpace = (id: string, section = "calendar") =>
    go(`/spaces/${id}/${section}`);
  const saveMyTaskReminders = async (taskId: string, offsets: number[]) => {
    try {
      const savedOffsets = await updateMyTaskReminders(
        accessToken,
        taskId,
        offsets,
      );
      setTasks((all) =>
        all.map((task) =>
          String(task.id) === taskId
            ? { ...task, myReminderOffsetsMinutes: savedOffsets }
            : task,
        ),
      );
      showToast("Your Task reminders were saved.");
    } catch {
      showToast("Your Task reminders couldn’t be saved. Try again.");
    }
  };
  const closeModal = () => {
    setModalState("");
    setEditingEvent(null);
    setEventEditScope("series");
    setEditingTask(null);
    setEditingList(null);
    setTaskListDefault("");
    setEventError("");
    setTaskError("");
    setPlaceError("");
    restoreFocus();
  };
  const relativeDay = (date: FormDataEntryValue | null) => {
    if (!date) return 1;
    const selectedDate = new Date(`${String(date)}T12:00:00`);
    const today = new Date();
    today.setHours(12, 0, 0, 0);
    return Math.round((selectedDate.getTime() - today.getTime()) / 86_400_000);
  };
  const submitModal = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const title = String(form.get("title") || "New item");
    if (modal === "event") {
      const date = String(form.get("date") || dateForOffset(calendarDay));
      const eventSpaceId = String(
        form.get("space") || editingEvent?.space || space.id,
      );
      const allDay = form.get("allDay") === "on";
      const time = String(form.get("time") || "18:00");
      const endTime = String(form.get("end") || "");
      const recurrenceValue = String(form.get("repeat") || "");
      const recurrenceEndsOnValue = String(form.get("recurrenceEndsOn") || "");
      const eventTimeZone = String(
        form.get("timeZone") || editingEvent?.timeZone || timeZone || "UTC",
      );
      let startsAt: string;
      let endsAt: string | null;
      try {
        startsAt = localDateTimeToRfc3339(
          date,
          allDay ? "00:00" : time,
          eventTimeZone,
        );
        endsAt =
          !allDay && endTime
            ? localDateTimeToRfc3339(date, endTime, eventTimeZone)
            : null;
      } catch {
        setEventError(
          "That local time doesn’t exist in this timezone. Choose another time or timezone.",
        );
        return;
      }
      const input: CalendarEventInput = {
        title,
        startsAt,
        endsAt,
        timeZone: eventTimeZone,
        allDay,
        location: String(form.get("place") || "").trim() || null,
        notes: String(form.get("notes") || "").trim() || null,
        recurrence:
          eventEditScope === "occurrence"
            ? null
            : recurrenceValue === "Every day"
              ? "daily"
              : recurrenceValue === "Every week"
                ? "weekly"
                : recurrenceValue === "Every month"
                  ? "monthly"
                  : null,
        recurrenceEndsOn:
          eventEditScope !== "occurrence" && recurrenceValue
            ? recurrenceEndsOnValue || null
            : null,
      };
      if (input.endsAt && new Date(input.endsAt) <= new Date(input.startsAt)) {
        setEventError("The end time must be after the start time.");
        return;
      }
      const eventSpace = activeSpaces.find((item) => item.id === eventSpaceId);
      if (!eventSpace) {
        setEventError(
          "Choose one of your account Spaces before adding an event.",
        );
        return;
      }
      if (!editingEvent && eventSpace.role === "Viewer") {
        setEventError(
          "Only an Editor or Owner can add Calendar events to this Space.",
        );
        return;
      }
      setSavingEvent(true);
      setEventError("");
      try {
        const saved = editingEvent
          ? eventEditScope === "occurrence" && editingEvent.occurrenceDate
            ? await updateCalendarOccurrence(
                accessToken,
                String(editingEvent.id),
                editingEvent.occurrenceDate,
                input,
              )
            : await updateCalendarEvent(
                accessToken,
                String(editingEvent.id),
                input,
              )
          : await createCalendarEvent(accessToken, eventSpaceId, input);
        if (editingEvent) {
          if (eventEditScope === "occurrence") {
            setEvents((current) =>
              current.map((item) =>
                item.id === saved.id &&
                item.occurrenceDate === editingEvent.occurrenceDate
                  ? saved
                  : item,
              ),
            );
            setEventsReload((current) => current + 1);
          } else if (editingEvent.repeat) {
            setEventsReload((current) => current + 1);
          } else {
            setEvents((current) =>
              current.map((item) => (item.id === saved.id ? saved : item)),
            );
          }
        } else {
          if (saved.repeat) setEventsReload((current) => current + 1);
          else setEvents((current) => [saved, ...current]);
        }
        setEventsStatus("ready");
        closeModal();
        showToast(
          editingEvent
            ? eventEditScope === "occurrence"
              ? "Calendar occurrence updated."
              : "Calendar series updated."
            : "Calendar event saved.",
        );
      } catch (error) {
        setEventError(
          error instanceof CalendarEventsApiError && error.status === 403
            ? "Only an Editor or Owner can change Calendar events in this Space."
            : error instanceof CalendarEventsApiError && error.status === 400
              ? "Check the event details and try again."
              : "The event couldn’t be saved. Check your connection and retry.",
        );
      } finally {
        setSavingEvent(false);
      }
      return;
    }
    if (modal === "space") {
      setCreatingSpace(true);
      setSpacesError("");
      try {
        const created = await createSharedSpace(accessToken, title);
        setServerSpaces((current) => [...current, created]);
        setSpacesStatus("ready");
        closeModal();
        go(`/spaces/${created.id}/calendar`);
        showToast("Shared Space created.");
      } catch (error) {
        setSpacesError(
          error instanceof SpacesApiError && error.status === 400
            ? "Enter a Space name with 1–80 characters."
            : "The Space couldn’t be created. Check your connection and try again.",
        );
      } finally {
        setCreatingSpace(false);
      }
      return;
    }
    if (modal === "task") {
      const taskSpaceId = editingTask?.space ?? space.id;
      const taskSpace = activeSpaces.find((item) => item.id === taskSpaceId);
      if (!taskSpace) {
        setTaskError("Choose one of your account Spaces before saving a Task.");
        return;
      }
      if (taskSpace.role === "Viewer") {
        setTaskError(taskPermissionError);
        return;
      }
      setSavingTask(true);
      setTaskError("");
      try {
        const listName = String(form.get("list") || "To-do").trim();
        const taskInput = {
          title,
          listName,
          dueDate: String(form.get("dueDate") || "") || null,
          dueTime: String(form.get("taskTime") || "") || null,
          assigneeId: String(form.get("assignee") || "") || null,
          reminderOffsetsMinutes: form.getAll("reminderOffset").map(Number),
        };
        const saved = editingTask
          ? await updateTodoTask(accessToken, String(editingTask.id), taskInput)
          : await createTodoTask(accessToken, space.id, taskInput);
        setTasks((all) =>
          editingTask
            ? all.map((task) => (task.id === saved.id ? saved : task))
            : [saved, ...all],
        );
        setTaskLists((all) => ({
          ...all,
          [space.id]: [...new Set([...(all[space.id] ?? []), listName])],
        }));
        if (saved.listId) {
          setServerTaskLists((all) => {
            const lists = all[space.id] ?? [];
            return {
              ...all,
              [space.id]: lists.some((list) => list.id === saved.listId)
                ? lists
                : [
                    ...lists,
                    {
                      id: saved.listId!,
                      space_id: space.id,
                      space_name: space.name,
                      space_kind:
                        space.tone === "personal" ? "personal" : "shared",
                      name: saved.list,
                    },
                  ],
            };
          });
        }
        setTasksStatus("ready");
        closeModal();
        showToast(editingTask ? "Task updated." : "Task saved to this Space.");
      } catch (error) {
        setTaskError(
          error instanceof TodosApiError && error.status === 403
            ? taskPermissionError
            : error instanceof TodosApiError && error.status === 400
              ? "Check the Task details and try again."
              : "The Task couldn’t be saved. Check your connection and retry.",
        );
      } finally {
        setSavingTask(false);
      }
      return;
    }
    if (modal === "place") {
      const place: PlaceItem = {
        name: title,
        address: String(form.get("address") || "Address not added"),
        category: String(form.get("category") || "Food & drink"),
        note: String(form.get("note") || "") || undefined,
        space: space.id,
      };
      const placeSpace = activeSpaces.find((item) => item.id === space.id);
      if (!placeSpace) {
        setPlaceError(
          "Choose one of your account Spaces before saving a place.",
        );
        return;
      }
      if (placeSpace.role === "Viewer") {
        setPlaceError("Only an Editor or Owner can save places in this Space.");
        return;
      }
      setSavingPlace(true);
      setPlaceError("");
      try {
        const saved = await createSavedPlace(accessToken, space.id, place);
        setPlaces((all) => [saved, ...all]);
        setPlacesStatus("ready");
        closeModal();
        showToast("Saved place added to this Space.");
      } catch (error) {
        setPlaceError(
          error instanceof PlacesApiError && error.status === 403
            ? "Only an Editor or Owner can save places in this Space."
            : error instanceof PlacesApiError && error.status === 400
              ? "Check the place name, category, and address."
              : "The place couldn’t be saved. Check your connection and retry.",
        );
      } finally {
        setSavingPlace(false);
      }
      return;
    }
    if (modal === "list") {
      if (space.role === "Viewer") {
        setTaskError(taskPermissionError);
        return;
      }
      const listName = title.trim();
      setSavingTask(true);
      setTaskError("");
      try {
        if (editingList) {
          const updated = await updateTodoList(
            accessToken,
            editingList.id,
            listName,
          );
          setServerTaskLists((all) => ({
            ...all,
            [space.id]: (all[space.id] ?? []).map((list) =>
              list.id === updated.id ? updated : list,
            ),
          }));
          setTaskLists((all) => ({
            ...all,
            [space.id]: (all[space.id] ?? []).map((name) =>
              name === editingList.name ? updated.name : name,
            ),
          }));
          setTasks((all) =>
            all.map((task) =>
              task.listId === updated.id
                ? { ...task, list: updated.name }
                : task,
            ),
          );
          closeModal();
          showToast("To-do list renamed.");
          return;
        }
        const created = await createTodoList(accessToken, space.id, listName);
        setServerTaskLists((all) => ({
          ...all,
          [space.id]: [...(all[space.id] ?? []), created],
        }));
        setTaskLists((all) => ({
          ...all,
          [space.id]: [...new Set([...(all[space.id] ?? []), created.name])],
        }));
        setTasksStatus("ready");
        closeModal();
        showToast("To-do list saved to this Space.");
      } catch (error) {
        setTaskError(
          error instanceof TodosApiError && error.status === 403
            ? taskPermissionError
            : error instanceof TodosApiError && error.status === 400
              ? "Enter a list name with 1–80 characters."
              : error instanceof TodosApiError && error.status === 409
                ? "A list with that name already exists."
                : "The list couldn’t be saved. Check your connection and retry.",
        );
      } finally {
        setSavingTask(false);
      }
      return;
    }
    closeModal();
  };

  const editSelectedEvent = (scope: "occurrence" | "series" = "series") => {
    if (!selected || "list" in selected) return;
    if (
      !activeSpaces.some(
        (item) => item.id === selected.space && item.role !== "Viewer",
      )
    ) {
      setEventError("Only an Editor or Owner can change Calendar events.");
      return;
    }
    setEditingEvent(selected);
    setEventEditScope(scope);
    setSelectedState(null);
    setEventError("");
    setModal("event");
  };

  const removeSelectedEvent = async (
    scope: "occurrence" | "series" = "series",
  ) => {
    if (!selected || "list" in selected) return;
    if (
      !activeSpaces.some(
        (item) => item.id === selected.space && item.role !== "Viewer",
      )
    ) {
      setEventError("Only an Editor or Owner can change Calendar events.");
      return;
    }
    if (typeof selected.id === "string") {
      try {
        if (scope === "occurrence" && selected.occurrenceDate) {
          await cancelCalendarOccurrence(
            accessToken,
            selected.id,
            selected.occurrenceDate,
          );
        } else {
          await deleteCalendarEvent(accessToken, selected.id);
        }
      } catch (error) {
        setEventError(
          error instanceof CalendarEventsApiError && error.status === 403
            ? "Only an Editor or Owner can change Calendar events in this Space."
            : "The Calendar event couldn’t be canceled. Check your connection and retry.",
        );
        return;
      }
    }
    setEvents((current) =>
      current.filter((item) =>
        scope === "occurrence" && selected.occurrenceDate
          ? !(
              item.id === selected.id &&
              item.occurrenceDate === selected.occurrenceDate
            )
          : item.id !== selected.id,
      ),
    );
    setSelected(null);
    setEventError("");
    showToast(
      scope === "occurrence"
        ? "Calendar occurrence canceled."
        : "Calendar event deleted.",
    );
  };

  const editSelectedTask = () => {
    if (!selected || !("list" in selected)) return;
    if (
      !activeSpaces.some(
        (item) => item.id === selected.space && item.role !== "Viewer",
      )
    ) {
      setTaskError(taskPermissionError);
      return;
    }
    setEditingTask(selected);
    setSelectedState(null);
    setTaskError("");
    setModal("task");
  };

  const removeSelectedTask = async () => {
    if (!selected || !("list" in selected)) return;
    const task = selected;
    if (
      !activeSpaces.some(
        (item) => item.id === task.space && item.role !== "Viewer",
      )
    ) {
      setTaskError(taskPermissionError);
      return;
    }
    requestConfirmation({
      title: "Delete this Task?",
      description: "This cannot be undone.",
      confirmLabel: "Delete Task",
      onConfirm: async () => {
        try {
          await deleteTodoTask(accessToken, String(task.id));
          setTasks((current) => current.filter((item) => item.id !== task.id));
          setSelected(null);
          setTasksReload((current) => current + 1);
          showToast("Task deleted.");
        } catch (error) {
          setTaskError(
            error instanceof TodosApiError && error.status === 403
              ? taskPermissionError
              : "The Task couldn’t be deleted. Check your connection and retry.",
          );
        }
      },
    });
  };

  const editTaskList = (listId: string) => {
    const list = serverTaskLists[space.id]?.find((item) => item.id === listId);
    if (!list) return;
    if (space.role === "Viewer") {
      setTaskError(taskPermissionError);
      return;
    }
    setEditingList(list);
    setTaskError("");
    setModal("list");
  };

  const removeTaskList = async (listId: string) => {
    const list = serverTaskLists[space.id]?.find((item) => item.id === listId);
    if (!list) return;
    if (space.role === "Viewer") {
      setTaskError(taskPermissionError);
      return;
    }
    requestConfirmation({
      title: `Delete “${list.name}”?`,
      description: "This will also delete all Tasks in this To-do list.",
      confirmLabel: "Delete To-do list",
      onConfirm: async () => {
        setTaskError("");
        try {
          await deleteTodoList(accessToken, listId);
          setTasks((current) =>
            current.filter((task) => task.listId !== listId),
          );
          setServerTaskLists((current) => ({
            ...current,
            [space.id]: (current[space.id] ?? []).filter(
              (item) => item.id !== listId,
            ),
          }));
          setTaskLists((current) => ({
            ...current,
            [space.id]: (current[space.id] ?? []).filter(
              (name) => name !== list.name,
            ),
          }));
          setTasksReload((current) => current + 1);
          showToast("To-do list and its Tasks deleted.");
        } catch (error) {
          setTaskError(
            error instanceof TodosApiError && error.status === 403
              ? taskPermissionError
              : "The To-do list couldn’t be deleted. Check your connection and retry.",
          );
        }
      },
    });
  };

  const createTaskList = async (name: string): Promise<boolean> => {
    const listName = name.trim();
    if (!listName) return false;
    if (space.role === "Viewer") {
      setTaskError(taskPermissionError);
      return false;
    }
    setSavingTask(true);
    setTaskError("");
    try {
      const created = await createTodoList(accessToken, space.id, listName);
      setServerTaskLists((all) => ({
        ...all,
        [space.id]: [...(all[space.id] ?? []), created],
      }));
      setTaskLists((all) => ({
        ...all,
        [space.id]: [...new Set([...(all[space.id] ?? []), created.name])],
      }));
      setTasksStatus("ready");
      showToast("To-do list saved to this Space.");
      return true;
    } catch (error) {
      setTaskError(
        error instanceof TodosApiError && error.status === 403
          ? taskPermissionError
          : error instanceof TodosApiError && error.status === 400
            ? "Enter a list name with 1–80 characters."
            : error instanceof TodosApiError && error.status === 409
              ? "A list with that name already exists."
              : "The list couldn’t be saved. Check your connection and retry.",
      );
      return false;
    } finally {
      setSavingTask(false);
    }
  };

  const removeSavedPlace = async (place: PlaceItem) => {
    if (!place.id || removingPlaceId) return;
    if (
      !activeSpaces.some(
        (item) => item.id === space.id && item.role !== "Viewer",
      )
    ) {
      setPlaceError("Only an Editor or Owner can remove places in this Space.");
      return;
    }
    setRemovingPlaceId(place.id);
    setPlaceError("");
    try {
      await deleteSavedPlace(accessToken, space.id, place.id);
      setPlaces((current) => current.filter((item) => item.id !== place.id));
      showToast("Saved place removed.");
    } catch (error) {
      setPlaceError(
        error instanceof PlacesApiError && error.status === 403
          ? "Only an Editor or Owner can remove places in this Space."
          : "The Saved place couldn’t be removed. Retry your action.",
      );
    } finally {
      setRemovingPlaceId(null);
    }
  };

  const loadMoreEvents = async () => {
    if (eventsNextOffset === null || loadingMoreEvents) return;
    setLoadingMoreEvents(true);
    setEventError("");
    try {
      const page = await listCalendarEvents(
        accessToken,
        eventsNextOffset,
        dateForOffset(0),
        dateForOffset(90),
      );
      setEvents((current) => [
        ...current,
        ...page.items.filter(
          (item) => !current.some((existing) => existing.id === item.id),
        ),
      ]);
      setEventsNextOffset(page.nextOffset);
    } catch {
      setEventError("More Calendar events couldn’t be loaded. Please retry.");
    } finally {
      setLoadingMoreEvents(false);
    }
  };

  const loadMoreTasks = async () => {
    if (tasksNextOffset === null || loadingMoreTasks) return;
    setLoadingMoreTasks(true);
    setTaskError("");
    try {
      const page = await listTodos(accessToken, 0, tasksNextOffset);
      setTasks((current) => [
        ...current,
        ...page.tasks.filter(
          (item) => !current.some((existing) => existing.id === item.id),
        ),
      ]);
      setTasksNextOffset(page.tasksNextOffset);
    } catch {
      setTaskError("More Tasks couldn’t be loaded. Please retry.");
    } finally {
      setLoadingMoreTasks(false);
    }
  };

  const loadMoreLists = async () => {
    if (listsNextOffset === null || loadingMoreLists) return;
    setLoadingMoreLists(true);
    setTaskError("");
    try {
      const page = await listTodos(accessToken, listsNextOffset, 0);
      setServerTaskLists((current) => {
        const next = { ...current };
        for (const list of page.lists) {
          next[list.space_id] = [
            ...(next[list.space_id] ?? []).filter(
              (item) => item.id !== list.id,
            ),
            list,
          ];
        }
        return next;
      });
      setTaskLists((current) => {
        const next = { ...current };
        for (const list of page.lists) {
          next[list.space_id] = [
            ...new Set([...(next[list.space_id] ?? []), list.name]),
          ];
        }
        return next;
      });
      setListsNextOffset(page.listsNextOffset);
    } catch {
      setTaskError("More To-do lists couldn’t be loaded. Please retry.");
    } finally {
      setLoadingMoreLists(false);
    }
  };

  const loadMorePlaces = async () => {
    if (placesNextOffset === null || loadingMorePlaces) return;
    setLoadingMorePlaces(true);
    setPlaceError("");
    try {
      const page = await listSavedPlaces(accessToken, placesNextOffset);
      setPlaces((current) => [
        ...current,
        ...page.items.filter(
          (item) => !current.some((existing) => existing.id === item.id),
        ),
      ]);
      setPlacesNextOffset(page.nextOffset);
    } catch {
      setPlaceError("More Saved places couldn’t be loaded. Please retry.");
    } finally {
      setLoadingMorePlaces(false);
    }
  };

  return {
    ...membershipState,
    ready,
    isOffline,
    activeSpaces,
    spacesStatus,
    spacesError,
    setSpacesError,
    retrySpaces: () => setSpacesReload((current) => current + 1),
    deletedSpaces,
    deletedSpacesStatus,
    creatingSpace,
    accessToken,
    path,
    reminderTime,
    setReminderTime,
    timeZone,
    setTimeZone,
    notificationPrefs,
    setNotificationPrefs,
    notificationPrefsStatus,
    notificationPrefsError,
    savePreferences,
    saveMyTaskReminders,
    mutedSpaces,
    setMutedSpaces,
    setSpaceMuted,
    events,
    setEvents,
    eventsStatus,
    eventError,
    setEventError,
    retryEvents: () => setEventsReload((current) => current + 1),
    eventsNextOffset,
    loadingMoreEvents,
    loadMoreEvents,
    savingEvent,
    editingEvent,
    eventEditScope,
    setEditingEvent,
    editSelectedEvent,
    removeSelectedEvent,
    tasks,
    setTasks,
    tasksStatus,
    taskError,
    retryTasks: () => setTasksReload((current) => current + 1),
    tasksNextOffset,
    listsNextOffset,
    loadingMoreTasks,
    loadingMoreLists,
    loadMoreTasks,
    loadMoreLists,
    savingTask,
    editingTask,
    editingList,
    editSelectedTask,
    removeSelectedTask,
    editTaskList,
    removeTaskList,
    createTaskList,
    placesStatus,
    placeError,
    retryPlaces: () => setPlacesReload((current) => current + 1),
    placesNextOffset,
    loadingMorePlaces,
    loadMorePlaces,
    savingPlace,
    removingPlaceId,
    removeSavedPlace,
    hidden,
    setHidden,
    filter,
    setFilter,
    taskListDefault,
    setTaskListDefault,
    modal,
    setModal,
    confirmation,
    requestConfirmation,
    dismissConfirmation,
    confirmPendingAction,
    selected,
    setSelected,
    placeFilter,
    setPlaceFilter,
    places,
    setPlaces,
    taskLists,
    setTaskLists,
    serverTaskLists,
    calendarDay,
    setCalendarDay,
    unread,
    notifications,
    notificationsStatus,
    notificationsError,
    notificationActionError,
    notificationsNextOffset,
    loadingMoreNotifications,
    retryNotifications,
    loadMoreNotifications,
    markNotificationRead,
    markAllNotificationsRead,
    openNotification: (notification: (typeof notifications)[number]) =>
      goSpace(notification.space_id, notification.destination),
    toast,
    mobileMenu,
    setMobileMenu,
    spaceSwitcherOpen,
    setSpaceSwitcherOpen,
    space,
    isSpacePage,
    routeSection,
    filteredEvents,
    filteredTasks,
    days,
    go,
    goSpace,
    showToast,
    toggleTask,
    closeModal,
    submitModal,
  };
}

export type PlannerAppState = ReturnType<typeof usePlannerApp>;
