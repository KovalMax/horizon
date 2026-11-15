import type { NotificationItem } from "../notificationsApi";
import type { PlannerAppState, PageHelpers } from "./context";
import { Button, Icon } from "../../../shared/ui";

function notificationIcon(eventType: string): string {
  if (eventType === "task_assignment") return "list";
  if (eventType === "event_change") return "calendar";
  return "users";
}

function formatTime(value: string): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function NotificationRow({
  item,
  state,
}: {
  item: NotificationItem;
  state: PlannerAppState;
}) {
  const isUnread = item.read_at === null;
  return (
    <article className={`notification-item ${isUnread ? "unread" : ""}`}>
      <span className="notification-icon" aria-hidden="true">
        <Icon name={notificationIcon(item.event_type)} />
      </span>
      <div className="notification-content">
        <h2>{item.title}</h2>
        <p>{item.body}</p>
        <small>{item.space_name}</small>
        <time dateTime={item.created_at}>{formatTime(item.created_at)}</time>
        <div className="notification-actions">
          <Button
            variant="outline"
            onClick={() => state.openNotification(item)}
          >
            Open{" "}
            {item.destination === "tasks"
              ? "Tasks"
              : item.destination === "calendar"
                ? "Calendar"
                : "Members"}
          </Button>
          <Button
            variant="quiet"
            onClick={() => void state.markNotificationRead(item, isUnread)}
          >
            Mark as {isUnread ? "read" : "unread"}
          </Button>
        </div>
      </div>
      {isUnread && <span className="unread-dot" aria-label="Unread" />}
    </article>
  );
}

export function NotificationsPage({
  state,
  helpers,
}: {
  state: PlannerAppState;
  helpers: PageHelpers;
}) {
  const { pageHeader } = helpers;
  return (
    <>
      {pageHeader(
        "Notifications",
        "Updates from your Spaces.",
        <Button
          variant="outline"
          disabled={state.unread === 0 || state.notificationsStatus !== "ready"}
          onClick={() => void state.markAllNotificationsRead()}
        >
          Mark all read
        </Button>,
      )}
      {state.notificationsStatus === "loading" && (
        <p role="status">Loading notifications…</p>
      )}
      {state.notificationsStatus === "error" && (
        <div role="alert">
          <p>{state.notificationsError}</p>
          <Button variant="outline" onClick={state.retryNotifications}>
            Retry
          </Button>
        </div>
      )}
      {state.notificationActionError && (
        <p role="alert">{state.notificationActionError}</p>
      )}
      {state.notificationsStatus === "ready" &&
        state.notifications.length === 0 && (
          <section className="list-card">
            <h2>You’re all caught up</h2>
            <p>New updates from your Spaces will appear here.</p>
          </section>
        )}
      {state.notificationsStatus === "ready" &&
        state.notifications.length > 0 && (
          <div className="notification-list" aria-label="Your notifications">
            {state.notifications.map((item) => (
              <NotificationRow key={item.id} item={item} state={state} />
            ))}
          </div>
        )}
      {state.notificationsNextOffset !== null && (
        <Button
          variant="outline"
          disabled={state.loadingMoreNotifications}
          onClick={() => void state.loadMoreNotifications()}
        >
          {state.loadingMoreNotifications ? "Loading…" : "Load more"}
        </Button>
      )}
    </>
  );
}
