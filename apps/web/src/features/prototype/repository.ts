import { seedEvents, seedTasks, type EventItem, type TaskItem } from "./domain";

export type PrototypeSnapshot = {
  events: EventItem[];
  tasks: TaskItem[];
};

/** Supplies fictional sample data for this browser session; it never persists changes. */
export interface PrototypeRepository {
  loadInitialSnapshot(): PrototypeSnapshot;
}

export class SessionPrototypeRepository implements PrototypeRepository {
  loadInitialSnapshot(): PrototypeSnapshot {
    return {
      events: seedEvents.map((event) => ({ ...event })),
      tasks: seedTasks.map((task) => ({ ...task })),
    };
  }
}
