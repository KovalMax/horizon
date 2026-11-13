import type { PrototypeAppState, PageHelpers } from "./context";
import { Icon, Button } from "../../../shared/ui";

export function NotificationsPage({
  state,
  helpers,
}: {
  state: PrototypeAppState;
  helpers: PageHelpers;
}) {
  const { setUnread, unread, goSpace, showToast } = state;
  const { pageHeader } = helpers;
  return (
    <>
      {pageHeader(
        "Notifications",
        "Updates from your Spaces.",
        <Button
          variant="outline"
          onClick={() => {
            setUnread(0);
            showToast("All notifications marked as read");
          }}
        >
          Mark all read
        </Button>,
      )}
      <div className="notification-list">
        {[
          {
            title: "Tomás Rivera asked to join",
            body: "Linden Street Home · Used the Viewer link LINDEN-7Q4X. Approve or decline in Members.",
            time: "45 minutes ago",
            icon: "users",
            space: "home",
          },
          {
            title: "Due today: Pick up dry cleaning",
            body: "Linden Street Home · Due at 17:00. Reminder set on the Task.",
            time: "90 minutes ago",
            icon: "list",
            space: "home",
          },
          {
            title: "Jonas assigned you a Task",
            body: "Renew home insurance in Home admin, due in 6 days.",
            time: "5 hours ago",
            icon: "list",
            space: "home",
          },
          {
            title: "Priya moved one Trip planning call",
            body: "Only the call on this date changed. Other weeks are unchanged.",
            time: "9 hours ago",
            icon: "calendar",
            space: "trip",
          },
        ].map((n, i) => (
          <button
            className={`notification-item ${i < unread ? "unread" : ""}`}
            key={n.title}
            onClick={() => {
              setUnread(Math.max(0, unread - 1));
              goSpace(
                n.space,
                i === 0 ? "members" : i === 1 || i === 2 ? "tasks" : "calendar",
              );
            }}
          >
            <span className="notification-icon">
              <Icon name={n.icon} />
            </span>
            <span>
              <b>{n.title}</b>
              <small>{n.body}</small>
              <small>{n.time}</small>
            </span>
            {i < unread && <i className="unread-dot" />}
          </button>
        ))}
      </div>
    </>
  );
}
