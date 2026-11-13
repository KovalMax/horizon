import type { EventItem, TaskItem } from "./domain";

export function createTask(input: {
  id: number;
  title: string;
  space: string;
  list: string;
  day?: number;
  time?: string;
  owner?: string;
}): TaskItem {
  return { ...input, day: input.day ?? 1 };
}

export function createEvent(input: {
  id: number;
  title: string;
  space: string;
  time: string;
  end: string;
  day?: number;
  allDay?: boolean;
  place?: string;
  repeat?: string;
}): EventItem {
  return { ...input, day: input.day ?? 1 };
}

export function toggleTaskCompletion(
  tasks: TaskItem[],
  id: TaskItem["id"],
): TaskItem[] {
  return tasks.map((task) =>
    task.id === id ? { ...task, done: !task.done } : task,
  );
}
