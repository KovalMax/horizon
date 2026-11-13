import { expect, test } from "vitest";
import {
  createEvent,
  createTask,
  toggleTaskCompletion,
} from "../src/features/prototype/application";
import { SessionPrototypeRepository } from "../src/features/prototype/repository";

test("prototype repository returns fictional session data without sharing mutable seeds", () => {
  const repository = new SessionPrototypeRepository();
  const first = repository.loadInitialSnapshot();
  first.tasks.length = 0;

  const next = repository.loadInitialSnapshot();
  expect(next.tasks.length).toBeGreaterThan(0);
  expect(next.events[0]?.title).toBeTruthy();
});

test("prototype application creates and toggles session-only tasks and events", () => {
  const task = createTask({
    id: 1,
    title: "Review plan",
    space: "home",
    list: "To-do",
  });
  const event = createEvent({
    id: 2,
    title: "Dinner",
    space: "home",
    time: "18:00",
    end: "19:00",
  });

  expect(task.day).toBe(1);
  expect(event.day).toBe(1);
  const completed = toggleTaskCompletion([task], task.id);
  expect(completed[0]?.done).toBe(true);
  expect(toggleTaskCompletion(completed, task.id)[0]?.done).toBe(false);
});
