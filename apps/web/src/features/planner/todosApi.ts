import { authenticatedFetch } from "../auth/api";
import type { TaskItem } from "./domain";

export type ApiTaskList = {
  id: string;
  space_id: string;
  space_name: string;
  space_kind: "personal" | "shared";
  name: string;
};

type ApiTask = {
  id: string;
  space_id: string;
  space_name: string;
  space_kind: "personal" | "shared";
  list_id: string;
  list_name: string;
  title: string;
  due_date: string | null;
  due_time: string | null;
  reminder_offsets_minutes: number[];
  my_reminder_offsets_minutes: number[];
  done: boolean;
  assignee_id: string | null;
  assignee_email: string | null;
};

type ApiTaskBoard = {
  lists: ApiTaskList[];
  tasks: ApiTask[];
  lists_next_offset: number | null;
  tasks_next_offset: number | null;
};

export class TodosApiError extends Error {
  constructor(public readonly status: number) {
    super("To-dos could not be loaded or saved.");
  }
}

function toTask(task: ApiTask): TaskItem {
  let day: number | undefined;
  if (task.due_date) {
    const [year, month, date] = task.due_date.split("-").map(Number);
    const due = Date.UTC(year!, month! - 1, date!);
    const today = new Date();
    const startOfToday = Date.UTC(
      today.getFullYear(),
      today.getMonth(),
      today.getDate(),
    );
    day = Math.round((due - startOfToday) / 86_400_000);
  }
  return {
    id: task.id,
    title: task.title,
    space: task.space_id,
    spaceName: task.space_name,
    spaceTone: task.space_kind === "personal" ? "personal" : "home",
    spaceIcon: task.space_kind === "personal" ? "person" : "home",
    list: task.list_name,
    listId: task.list_id,
    day,
    time: task.due_time?.slice(0, 5) ?? undefined,
    reminders: (task.reminder_offsets_minutes ?? []).length,
    reminderOffsetsMinutes: task.reminder_offsets_minutes ?? [1440, 60],
    myReminderOffsetsMinutes: task.my_reminder_offsets_minutes ??
      task.reminder_offsets_minutes ?? [1440, 60],
    done: task.done,
    assigneeId: task.assignee_id ?? undefined,
    assigneeEmail: task.assignee_email ?? undefined,
  };
}

async function readJson<T>(response: Response): Promise<T> {
  if (!response.ok) throw new TodosApiError(response.status);
  return (await response.json()) as T;
}

function headers(accessToken: string) {
  return {
    Authorization: `Bearer ${accessToken}`,
    "Content-Type": "application/json",
  };
}

export async function listTodos(
  accessToken: string,
  listsOffset = 0,
  tasksOffset = 0,
): Promise<{
  lists: ApiTaskList[];
  tasks: TaskItem[];
  listsNextOffset: number | null;
  tasksNextOffset: number | null;
}> {
  const board = await readJson<ApiTaskBoard>(
    await authenticatedFetch(
      `/api/tasks?lists_offset=${listsOffset}&tasks_offset=${tasksOffset}`,
      {
        headers: { Authorization: `Bearer ${accessToken}` },
      },
    ),
  );
  return {
    lists: board.lists,
    tasks: board.tasks.map(toTask),
    listsNextOffset: board.lists_next_offset,
    tasksNextOffset: board.tasks_next_offset,
  };
}

export async function createTodoList(
  accessToken: string,
  spaceId: string,
  name: string,
): Promise<ApiTaskList> {
  return readJson<ApiTaskList>(
    await authenticatedFetch(`/api/spaces/${spaceId}/lists`, {
      method: "POST",
      headers: headers(accessToken),
      body: JSON.stringify({ name }),
    }),
  );
}

export async function updateTodoList(
  accessToken: string,
  listId: string,
  name: string,
): Promise<ApiTaskList> {
  return readJson<ApiTaskList>(
    await authenticatedFetch(`/api/lists/${listId}`, {
      method: "PUT",
      headers: headers(accessToken),
      body: JSON.stringify({ name }),
    }),
  );
}

export async function deleteTodoList(
  accessToken: string,
  listId: string,
): Promise<void> {
  const response = await authenticatedFetch(`/api/lists/${listId}`, {
    method: "DELETE",
    headers: headers(accessToken),
  });
  if (!response.ok) throw new TodosApiError(response.status);
}

export async function createTodoTask(
  accessToken: string,
  spaceId: string,
  input: {
    title: string;
    listName: string;
    dueDate: string | null;
    dueTime: string | null;
    assigneeId: string | null;
    reminderOffsetsMinutes: number[];
  },
): Promise<TaskItem> {
  const task = await readJson<ApiTask>(
    await authenticatedFetch(`/api/spaces/${spaceId}/tasks`, {
      method: "POST",
      headers: headers(accessToken),
      body: JSON.stringify({
        title: input.title,
        list_name: input.listName,
        due_date: input.dueDate,
        due_time: input.dueTime,
        assignee_id: input.assigneeId,
        reminder_offsets_minutes: input.reminderOffsetsMinutes,
      }),
    }),
  );
  return toTask(task);
}

export async function updateTodoCompletion(
  accessToken: string,
  taskId: string,
  done: boolean,
): Promise<TaskItem> {
  const task = await readJson<ApiTask>(
    await authenticatedFetch(`/api/tasks/${taskId}/completion`, {
      method: "PUT",
      headers: headers(accessToken),
      body: JSON.stringify({ done }),
    }),
  );
  return toTask(task);
}

export async function updateTodoTask(
  accessToken: string,
  taskId: string,
  input: {
    title: string;
    listName: string;
    dueDate: string | null;
    dueTime: string | null;
    assigneeId: string | null;
    reminderOffsetsMinutes: number[];
  },
): Promise<TaskItem> {
  const task = await readJson<ApiTask>(
    await authenticatedFetch(`/api/tasks/${taskId}`, {
      method: "PUT",
      headers: headers(accessToken),
      body: JSON.stringify({
        title: input.title,
        list_name: input.listName,
        due_date: input.dueDate,
        due_time: input.dueTime,
        assignee_id: input.assigneeId,
        reminder_offsets_minutes: input.reminderOffsetsMinutes,
      }),
    }),
  );
  return toTask(task);
}

export async function deleteTodoTask(
  accessToken: string,
  taskId: string,
): Promise<void> {
  const response = await authenticatedFetch(`/api/tasks/${taskId}`, {
    method: "DELETE",
    headers: headers(accessToken),
  });
  if (!response.ok) throw new TodosApiError(response.status);
}

export async function updateMyTaskReminders(
  accessToken: string,
  taskId: string,
  offsetsMinutes: number[],
): Promise<number[]> {
  const result = await readJson<{ offsets_minutes: number[] }>(
    await authenticatedFetch(`/api/tasks/${taskId}/reminders`, {
      method: "PUT",
      headers: headers(accessToken),
      body: JSON.stringify({ offsets_minutes: offsetsMinutes }),
    }),
  );
  return result.offsets_minutes;
}
