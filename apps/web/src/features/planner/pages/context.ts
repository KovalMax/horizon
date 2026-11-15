import type { ReactNode } from "react";
import type { EventItem, Space, TaskItem } from "../domain";
import type { PlannerAppState } from "../usePlannerApp";

export type PageHelpers = {
  pageHeader: (
    title: string,
    description?: string,
    action?: ReactNode,
  ) => ReactNode;
  spaceCards: ReactNode;
  taskRow: (task: TaskItem, withSpace?: boolean) => ReactNode;
  eventRow: (event: EventItem, withSpace?: boolean) => ReactNode;
};
export type { PlannerAppState, Space };
