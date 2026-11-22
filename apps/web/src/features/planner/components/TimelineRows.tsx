import type { ReactNode } from "react";
import {
  dateLabel,
  type EventItem,
  type Space,
  type TaskItem,
} from "../domain";
import { Icon, SpacePill } from "../../../shared/ui";

function resolveSpace(
  id: string,
  name = "Space",
  tone = "home",
  icon = "home",
): Space {
  return { id, name, role: "Viewer", members: 0, tone, icon };
}

export function PageHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <header className="page-header">
      <div>
        <h1 tabIndex={-1}>{title}</h1>
        {description && <p>{description}</p>}
      </div>
      {action}
    </header>
  );
}

export function TaskRow({
  task,
  withSpace = false,
  canToggle = true,
  onToggle,
  onSelect,
}: {
  task: TaskItem;
  withSpace?: boolean;
  canToggle?: boolean;
  onToggle: (id: TaskItem["id"]) => void;
  onSelect: (task: TaskItem) => void;
}) {
  const space = resolveSpace(
    task.space,
    task.spaceName,
    task.spaceTone,
    task.spaceIcon,
  );
  return (
    <div className="task-row" key={task.id}>
      <button
        className={`check-circle ${task.done ? "checked" : ""}`}
        aria-label={
          task.done ? `Mark ${task.title} open` : `Complete ${task.title}`
        }
        disabled={!canToggle}
        onClick={() => onToggle(task.id)}
      >
        {task.done && <Icon name="check" size={15} />}
      </button>
      <button
        className={`task-name ${task.done ? "completed" : ""}`}
        onClick={() => onSelect(task)}
      >
        {task.title}
      </button>
      <div className="task-meta">
        {withSpace && <SpacePill space={space} />}
        {task.assigneeEmail && (
          <span>
            <Icon name="person" size={13} />
            {task.assigneeEmail}
          </span>
        )}
        {task.day !== undefined && (
          <span>
            <Icon name="calendar" size={13} />
            {task.day === 0
              ? "Today"
              : task.day === 1
                ? "Tomorrow"
                : dateLabel(task.day)}
          </span>
        )}
        {task.time && (
          <span>
            <Icon name="clock" size={13} />
            {task.time}
          </span>
        )}
        {task.reminders ? (
          <span>
            <Icon name="bell" size={13} />
            {task.reminders} reminder{task.reminders > 1 ? "s" : ""}
          </span>
        ) : null}
      </div>
    </div>
  );
}

export function EventRow({
  event,
  withSpace = false,
  onSelect,
}: {
  event: EventItem;
  withSpace?: boolean;
  onSelect: (event: EventItem) => void;
}) {
  const space = resolveSpace(
    event.space,
    event.spaceName,
    event.spaceTone,
    event.spaceIcon,
  );
  return (
    <div className={`event-row ${space.tone}`} key={event.id}>
      <div className="event-time">
        {event.allDay ? (
          "All day"
        ) : (
          <>
            {event.time}
            <small>{event.end}</small>
          </>
        )}
      </div>
      <div className="event-detail">
        <button onClick={() => onSelect(event)}>{event.title}</button>
        <div className="event-meta">
          {withSpace && <SpacePill space={space} />}
          {event.place && (
            <span>
              <Icon name="pin" size={13} />
              {event.place}
            </span>
          )}
          {event.repeat && (
            <span>
              <Icon name="repeat" size={13} />
              {event.repeat}
            </span>
          )}
          {event.zone && (
            <span>
              <Icon name="globe" size={13} />
              {event.zone}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
