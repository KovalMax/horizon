import { useEffect, useMemo, useState, type FormEvent } from "react";
import {
  places as seedPlaces,
  spaces,
  type EventItem,
  type PlaceItem,
  type TaskItem,
} from "./domain";
import { createEvent, createTask, toggleTaskCompletion } from "./application";
import { SessionPrototypeRepository } from "./repository";
import { checkServiceReadiness } from "../../shared/api/health";
import { createSharedSpace, listSpaces, SpacesApiError } from "./spacesApi";
import {
  CalendarEventsApiError,
  createCalendarEvent,
  deleteCalendarEvent,
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
  type ApiTaskList,
} from "./todosApi";
import { createSavedPlace, listSavedPlaces, PlacesApiError } from "./placesApi";
import {
  acceptSpaceInvitation,
  createSpaceInvitation,
  listMyInvitations,
  listSpaceInvitations,
  listSpaceMembers,
  MembersApiError,
  type SpaceInvitation,
  type SpaceMember,
} from "./membersApi";

type ModalKind = "event" | "task" | "place" | "space" | "list" | "";

function dateForOffset(offset: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offset);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

function localDateTimeToRfc3339(date: string, time: string): string {
  const value = new Date(`${date}T${time}`);
  if (Number.isNaN(value.getTime()))
    throw new Error("Invalid local date and time");
  return value.toISOString();
}

export function usePrototypeApp(accessToken: string | null = null) {
  const [ready, setReady] = useState<"checking" | "available" | "unavailable">(
    "checking",
  );
  const [path, setPath] = useState(window.location.pathname);
  const [spaceNames, setSpaceNames] = useState<Record<string, string>>({});
  const [serverSpaces, setServerSpaces] = useState<typeof spaces>([]);
  const [spacesStatus, setSpacesStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [spacesReload, setSpacesReload] = useState(0);
  const [spacesError, setSpacesError] = useState("");
  const [creatingSpace, setCreatingSpace] = useState(false);
  const [reminderTime, setReminderTime] = useState("09:00");
  const [notificationPrefs, setNotificationPrefs] = useState<
    Record<string, boolean>
  >({
    "Task assignments:In-app": true,
    "Task assignments:Email": true,
    "Due reminders:In-app": true,
    "Due reminders:Email": true,
    "Event changes:In-app": true,
    "Event changes:Email": false,
    "Membership changes:In-app": true,
    "Membership changes:Email": false,
    "Join requests:In-app": true,
    "Join requests:Email": true,
    "Join request results:In-app": true,
    "Join request results:Email": true,
  });
  const [mutedSpaces, setMutedSpaces] = useState<string[]>([]);
  const [repository] = useState(() => new SessionPrototypeRepository());
  const [initialData] = useState(() => repository.loadInitialSnapshot());
  const [events, setEvents] = useState(initialData.events);
  const [eventsStatus, setEventsStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [eventsReload, setEventsReload] = useState(0);
  const [eventError, setEventError] = useState("");
  const [eventsNextOffset, setEventsNextOffset] = useState<number | null>(null);
  const [loadingMoreEvents, setLoadingMoreEvents] = useState(false);
  const [savingEvent, setSavingEvent] = useState(false);
  const [editingEvent, setEditingEvent] = useState<EventItem | null>(null);
  const [tasks, setTasks] = useState(initialData.tasks);
  const [tasksStatus, setTasksStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [tasksReload, setTasksReload] = useState(0);
  const [taskError, setTaskError] = useState("");
  const [listsNextOffset, setListsNextOffset] = useState<number | null>(null);
  const [tasksNextOffset, setTasksNextOffset] = useState<number | null>(null);
  const [loadingMoreLists, setLoadingMoreLists] = useState(false);
  const [loadingMoreTasks, setLoadingMoreTasks] = useState(false);
  const [savingTask, setSavingTask] = useState(false);
  const [editingTask, setEditingTask] = useState<TaskItem | null>(null);
  const [editingList, setEditingList] = useState<ApiTaskList | null>(null);
  const [places, setPlaces] = useState(seedPlaces);
  const [placesStatus, setPlacesStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [placesReload, setPlacesReload] = useState(0);
  const [placeError, setPlaceError] = useState("");
  const [placesNextOffset, setPlacesNextOffset] = useState<number | null>(null);
  const [loadingMorePlaces, setLoadingMorePlaces] = useState(false);
  const [savingPlace, setSavingPlace] = useState(false);
  const [spaceMembers, setSpaceMembers] = useState<SpaceMember[]>([]);
  const [spaceInvitations, setSpaceInvitations] = useState<SpaceInvitation[]>(
    [],
  );
  const [myInvitations, setMyInvitations] = useState<SpaceInvitation[]>([]);
  const [membershipStatus, setMembershipStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [membershipError, setMembershipError] = useState("");
  const [membershipReload, setMembershipReload] = useState(0);
  const [savingInvitation, setSavingInvitation] = useState(false);
  const [acceptingInvitation, setAcceptingInvitation] = useState("");
  const [taskLists, setTaskLists] = useState<Record<string, string[]>>(() =>
    Object.fromEntries(
      spaces.map((item) => [
        item.id,
        [
          ...new Set(
            initialData.tasks
              .filter((task) => task.space === item.id)
              .map((task) => task.list),
          ),
        ],
      ]),
    ),
  );
  const [serverTaskLists, setServerTaskLists] = useState<
    Record<string, ApiTaskList[]>
  >({});
  const [hidden, setHidden] = useState<string[]>([]);
  const [filter, setFilter] = useState("all");
  const [doneOpen, setDoneOpen] = useState(true);
  const [modal, setModal] = useState<ModalKind>("");
  const [selected, setSelected] = useState<EventItem | TaskItem | null>(null);
  const [mapMode, setMapMode] = useState(false);
  const [placeFilter, setPlaceFilter] = useState("All");
  const [calendarDay, setCalendarDay] = useState(0);
  const [calendarExpanded, setCalendarExpanded] = useState(false);
  const [unread, setUnread] = useState(3);
  const [requestPending, setRequestPending] = useState(true);
  const [approved, setApproved] = useState(false);
  const [memberRoles, setMemberRoles] = useState<Record<string, string>>({});
  const [invitedEmails, setInvitedEmails] = useState<string[]>([
    "lena.fischer@example.com",
  ]);
  const [toast, setToast] = useState("");
  const [mobileMenu, setMobileMenu] = useState(false);
  const [spaceSwitcherOpen, setSpaceSwitcherOpen] = useState(false);
  const currentSpaceId =
    path.match(/^\/spaces\/([^/]+)/)?.[1] ?? serverSpaces[0]?.id ?? "";

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
    if (!accessToken) {
      setServerSpaces([]);
      setSpacesStatus("idle");
      setSpacesError("");
      return;
    }
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
    if (!accessToken) {
      setEvents(initialData.events);
      setEventsNextOffset(null);
      setEventsStatus("idle");
      setEventError("");
      return;
    }
    let active = true;
    setEvents([]);
    setEventsNextOffset(null);
    setEventsStatus("loading");
    setEventError("");
    listCalendarEvents(accessToken)
      .then((result) => {
        if (!active) return;
        setEvents(result.items);
        setEventsNextOffset(result.nextOffset);
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
  }, [accessToken, eventsReload, initialData.events]);

  useEffect(() => {
    if (!accessToken) {
      setTasks(initialData.tasks);
      setServerTaskLists({});
      setListsNextOffset(null);
      setTasksNextOffset(null);
      setTaskLists(
        Object.fromEntries(
          spaces.map((item) => [
            item.id,
            [
              ...new Set(
                initialData.tasks
                  .filter((task) => task.space === item.id)
                  .map((task) => task.list),
              ),
            ],
          ]),
        ),
      );
      setTasksStatus("idle");
      setTaskError("");
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
  }, [accessToken, tasksReload, initialData.tasks]);

  useEffect(() => {
    if (!accessToken) {
      setPlaces(seedPlaces);
      setPlacesNextOffset(null);
      setPlacesStatus("idle");
      setPlaceError("");
      return;
    }
    let active = true;
    setPlaces([]);
    setPlacesNextOffset(null);
    setPlacesStatus("loading");
    setPlaceError("");
    listSavedPlaces(accessToken)
      .then((result) => {
        if (!active) return;
        setPlaces(result.items);
        setPlacesNextOffset(result.nextOffset);
        setPlacesStatus("ready");
      })
      .catch(() => {
        if (!active) return;
        setPlacesStatus("error");
        setPlaceError(
          "Saved places couldn’t be loaded. Check your connection and retry.",
        );
      });
    return () => {
      active = false;
    };
  }, [accessToken, placesReload]);

  useEffect(() => {
    if (!accessToken) {
      setSpaceMembers([]);
      setSpaceInvitations([]);
      setMyInvitations([]);
      setMembershipStatus("idle");
      return;
    }
    const controller = new AbortController();
    setMembershipStatus("loading");
    setMembershipError("");
    Promise.all([
      currentSpaceId
        ? listSpaceMembers(accessToken, currentSpaceId)
        : Promise.resolve([]),
      currentSpaceId
        ? listSpaceInvitations(accessToken, currentSpaceId).catch(() => [])
        : Promise.resolve([]),
      listMyInvitations(accessToken),
    ])
      .then(([members, invitations, mine]) => {
        if (controller.signal.aborted) return;
        setSpaceMembers(members);
        setSpaceInvitations(invitations);
        setMyInvitations(mine);
        setMembershipStatus("ready");
      })
      .catch(() => {
        if (controller.signal.aborted) return;
        setMembershipStatus("error");
        setMembershipError(
          "Space Members couldn’t be loaded. Check your connection and retry.",
        );
      });
    return () => controller.abort();
  }, [accessToken, currentSpaceId, membershipReload]);

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
    setPath(to);
    setMobileMenu(false);
    setSpaceSwitcherOpen(false);
    window.scrollTo(0, 0);
  };

  const selectedSpaceId = path.match(/^\/spaces\/([^/]+)/)?.[1] ?? "";
  const activeSpaces = accessToken ? serverSpaces : spaces;
  const baseSpace =
    activeSpaces.find((item) => item.id === selectedSpaceId) ??
    activeSpaces[0] ??
    spaces[1]!;
  const space = {
    ...baseSpace,
    name: spaceNames[baseSpace.id] ?? baseSpace.name,
  };
  const isSpacePage = path.startsWith("/spaces/");
  const routeSection = path.split("/")[3] ?? "calendar";
  const belongsToCurrentSpaces = (spaceId: string) =>
    !accessToken || activeSpaces.some((item) => item.id === spaceId);
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

  const showToast = (message: string) => setToast(message);
  const inviteMember = async (email: string) => {
    if (!accessToken || !currentSpaceId) return false;
    setSavingInvitation(true);
    setMembershipError("");
    try {
      const invitation = await createSpaceInvitation(
        accessToken,
        currentSpaceId,
        email,
      );
      setSpaceInvitations((all) => [invitation, ...all]);
      showToast(`Invitation sent to ${invitation.email}.`);
      return true;
    } catch (error) {
      setMembershipError(
        error instanceof MembersApiError && error.status === 403
          ? "Only Owners and Editors can invite people."
          : error instanceof MembersApiError && error.status === 409
            ? "This email is already a Member or has a pending invitation."
            : "The invitation couldn’t be sent. Check the email address and try again.",
      );
      return false;
    } finally {
      setSavingInvitation(false);
    }
  };
  const acceptInvitation = async (invitationId: string) => {
    if (!accessToken) return;
    setAcceptingInvitation(invitationId);
    setMembershipError("");
    try {
      const accepted = await acceptSpaceInvitation(accessToken, invitationId);
      setMyInvitations((all) =>
        all.filter((invitation) => invitation.id !== invitationId),
      );
      setMembershipReload((current) => current + 1);
      setSpacesReload((current) => current + 1);
      showToast(`You joined ${accepted.space_name} as a Viewer.`);
    } catch {
      setMembershipError(
        "This invitation can’t be accepted with the signed-in email.",
      );
    } finally {
      setAcceptingInvitation("");
    }
  };
  const toggleTask = (id: TaskItem["id"]) => {
    const task = tasks.find((item) => item.id === id);
    if (accessToken && task && typeof id === "string") {
      setTaskError("");
      updateTodoCompletion(accessToken, id, !task.done)
        .then((updated) =>
          setTasks((all) =>
            all.map((item) => (item.id === id ? updated : item)),
          ),
        )
        .catch(() =>
          setTaskError(
            "The Task couldn’t be updated. Check your connection and retry.",
          ),
        );
      return;
    }
    setTasks((all) => toggleTaskCompletion(all, id));
  };
  const goSpace = (id: string, section = "calendar") =>
    go(`/spaces/${id}/${section}`);
  const closeModal = () => {
    setModal("");
    setEditingEvent(null);
    setEditingTask(null);
    setEditingList(null);
    setEventError("");
    setTaskError("");
    setPlaceError("");
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
      const allDay = form.get("allDay") === "on";
      const time = String(form.get("time") || "18:00");
      const endTime = String(form.get("end") || "");
      const recurrenceValue = String(form.get("repeat") || "");
      const timeZone =
        editingEvent?.timeZone ??
        Intl.DateTimeFormat().resolvedOptions().timeZone ??
        "UTC";
      const input: CalendarEventInput = {
        title,
        startsAt: localDateTimeToRfc3339(date, allDay ? "00:00" : time),
        endsAt:
          !allDay && endTime ? localDateTimeToRfc3339(date, endTime) : null,
        timeZone,
        allDay,
        location: String(form.get("place") || "").trim() || null,
        notes: String(form.get("notes") || "").trim() || null,
        recurrence:
          recurrenceValue === "Every day"
            ? "daily"
            : recurrenceValue === "Every week"
              ? "weekly"
              : recurrenceValue === "Every month"
                ? "monthly"
                : null,
      };
      if (input.endsAt && new Date(input.endsAt) <= new Date(input.startsAt)) {
        setEventError("The end time must be after the start time.");
        return;
      }
      if (accessToken) {
        if (!activeSpaces.some((item) => item.id === space.id)) {
          setEventError(
            "Choose one of your account Spaces before adding an event.",
          );
          return;
        }
        setSavingEvent(true);
        setEventError("");
        try {
          const saved = editingEvent
            ? await updateCalendarEvent(
                accessToken,
                String(editingEvent.id),
                input,
              )
            : await createCalendarEvent(accessToken, space.id, input);
          setEvents((current) =>
            editingEvent
              ? current.map((item) => (item.id === saved.id ? saved : item))
              : [saved, ...current],
          );
          setEventsStatus("ready");
          closeModal();
          showToast(
            editingEvent ? "Calendar event updated." : "Calendar event saved.",
          );
        } catch (error) {
          setEventError(
            error instanceof CalendarEventsApiError && error.status === 400
              ? "Check the event details and try again."
              : "The event couldn’t be saved. Check your connection and retry.",
          );
        } finally {
          setSavingEvent(false);
        }
        return;
      }
      const sessionEvent = createEvent({
        id:
          editingEvent && typeof editingEvent.id === "number"
            ? editingEvent.id
            : Date.now(),
        title,
        space: space.id,
        time,
        end: endTime || "19:00",
        day: relativeDay(date),
        allDay,
        place: input.location ?? undefined,
        repeat: recurrenceValue || undefined,
      });
      setEvents((current) =>
        editingEvent
          ? current.map((item) =>
              item.id === editingEvent.id ? sessionEvent : item,
            )
          : [sessionEvent, ...current],
      );
      closeModal();
      showToast(
        editingEvent
          ? "Event updated (prototype only)."
          : "Event added (prototype only).",
      );
      return;
    }
    if (modal === "space") {
      if (!accessToken) {
        setSpacesError("Sign in to create a shared Space.");
        return;
      }
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
      if (accessToken) {
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
          };
          const saved = editingTask
            ? await updateTodoTask(
                accessToken,
                String(editingTask.id),
                taskInput,
              )
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
          showToast(
            editingTask ? "Task updated." : "Task saved to this Space.",
          );
        } catch (error) {
          setTaskError(
            error instanceof TodosApiError && error.status === 400
              ? "Check the Task details and try again."
              : "The Task couldn’t be saved. Check your connection and retry.",
          );
        } finally {
          setSavingTask(false);
        }
        return;
      }
      setTasks((all) => [
        createTask({
          id: Date.now(),
          title,
          space: space.id,
          list: String(form.get("list") || "To-do"),
          day: form.get("dueDate")
            ? relativeDay(form.get("dueDate"))
            : undefined,
          time: String(form.get("taskTime") || "") || undefined,
          owner: String(form.get("owner") || "") || undefined,
        }),
        ...all,
      ]);
    }
    if (modal === "place") {
      const place: PlaceItem = {
        name: title,
        address: String(form.get("address") || "Address not added"),
        category: String(form.get("category") || "Food & drink"),
        note: String(form.get("note") || "") || undefined,
        space: space.id,
      };
      if (accessToken) {
        if (!activeSpaces.some((item) => item.id === space.id)) {
          setPlaceError(
            "Choose one of your account Spaces before saving a place.",
          );
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
            error instanceof PlacesApiError && error.status === 400
              ? "Check the place name, category, and address."
              : "The place couldn’t be saved. Check your connection and retry.",
          );
        } finally {
          setSavingPlace(false);
        }
        return;
      }
      setPlaces((all) => [place, ...all]);
    }
    if (modal === "list") {
      const listName = title.trim();
      if (accessToken) {
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
            error instanceof TodosApiError && error.status === 400
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
      setTaskLists((all) => ({
        ...all,
        [space.id]: editingList
          ? (all[space.id] ?? []).map((name) =>
              name === editingList.name ? listName : name,
            )
          : [...new Set([...(all[space.id] ?? []), listName])],
      }));
    }
    closeModal();
    showToast(
      `${modal === "task" ? "Task" : modal === "place" ? "Place" : "Space"} added (prototype only).`,
    );
  };

  const editSelectedEvent = () => {
    if (!selected || "list" in selected) return;
    setEditingEvent(selected);
    setSelected(null);
    setEventError("");
    setModal("event");
  };

  const removeSelectedEvent = async () => {
    if (!selected || "list" in selected) return;
    if (accessToken && typeof selected.id === "string") {
      try {
        await deleteCalendarEvent(accessToken, selected.id);
      } catch {
        setEventError(
          "The event couldn’t be deleted. Check your connection and retry.",
        );
        return;
      }
    }
    setEvents((current) => current.filter((item) => item.id !== selected.id));
    setSelected(null);
    setEventError("");
    showToast(
      accessToken
        ? "Calendar event deleted."
        : "Event deleted (prototype only).",
    );
  };

  const editSelectedTask = () => {
    if (!selected || !("list" in selected) || !accessToken) return;
    setEditingTask(selected);
    setSelected(null);
    setTaskError("");
    setModal("task");
  };

  const removeSelectedTask = async () => {
    if (!selected || !("list" in selected) || !accessToken) return;
    if (!window.confirm("Delete this Task? This cannot be undone.")) return;
    try {
      await deleteTodoTask(accessToken, String(selected.id));
      setTasks((current) => current.filter((task) => task.id !== selected.id));
      setSelected(null);
      setTasksReload((current) => current + 1);
      showToast("Task deleted.");
    } catch {
      setTaskError(
        "The Task couldn’t be deleted. Check your connection and retry.",
      );
    }
  };

  const editTaskList = (listId: string) => {
    const list = serverTaskLists[space.id]?.find((item) => item.id === listId);
    if (!list) return;
    setEditingList(list);
    setTaskError("");
    setModal("list");
  };

  const removeTaskList = async (listId: string) => {
    if (!accessToken) return;
    const list = serverTaskLists[space.id]?.find((item) => item.id === listId);
    if (!list) return;
    if (
      !window.confirm(
        `Delete “${list.name}” and all Tasks in it? This cannot be undone.`,
      )
    ) {
      return;
    }
    setTaskError("");
    try {
      await deleteTodoList(accessToken, listId);
      setTasks((current) => current.filter((task) => task.listId !== listId));
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
    } catch {
      setTaskError(
        "The To-do list couldn’t be deleted. Check your connection and retry.",
      );
    }
  };

  const loadMoreEvents = async () => {
    if (!accessToken || eventsNextOffset === null || loadingMoreEvents) return;
    setLoadingMoreEvents(true);
    setEventError("");
    try {
      const page = await listCalendarEvents(accessToken, eventsNextOffset);
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
    if (!accessToken || tasksNextOffset === null || loadingMoreTasks) return;
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
    if (!accessToken || listsNextOffset === null || loadingMoreLists) return;
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
    if (!accessToken || placesNextOffset === null || loadingMorePlaces) return;
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
    ready,
    activeSpaces,
    spacesStatus,
    spacesError,
    setSpacesError,
    retrySpaces: () => setSpacesReload((current) => current + 1),
    creatingSpace,
    accessToken,
    path,
    spaceNames,
    setSpaceNames,
    reminderTime,
    setReminderTime,
    notificationPrefs,
    setNotificationPrefs,
    mutedSpaces,
    setMutedSpaces,
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
    placesStatus,
    placeError,
    retryPlaces: () => setPlacesReload((current) => current + 1),
    placesNextOffset,
    loadingMorePlaces,
    loadMorePlaces,
    savingPlace,
    spaceMembers,
    spaceInvitations,
    myInvitations,
    membershipStatus,
    membershipError,
    savingInvitation,
    acceptingInvitation,
    inviteMember,
    acceptInvitation,
    hidden,
    setHidden,
    filter,
    setFilter,
    doneOpen,
    setDoneOpen,
    modal,
    setModal,
    selected,
    setSelected,
    mapMode,
    setMapMode,
    placeFilter,
    setPlaceFilter,
    places,
    setPlaces,
    taskLists,
    setTaskLists,
    serverTaskLists,
    calendarDay,
    setCalendarDay,
    calendarExpanded,
    setCalendarExpanded,
    unread,
    setUnread,
    requestPending,
    setRequestPending,
    approved,
    setApproved,
    memberRoles,
    setMemberRoles,
    invitedEmails,
    setInvitedEmails,
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

export type PrototypeAppState = ReturnType<typeof usePrototypeApp>;
