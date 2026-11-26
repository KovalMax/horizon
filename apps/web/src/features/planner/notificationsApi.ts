import { authenticatedFetch } from "../auth/api";

export type NotificationItem = {
  id: string;
  space_id: string;
  space_name: string;
  event_type: string;
  title: string;
  body: string;
  destination: "tasks" | "calendar" | "members";
  created_at: string;
  read_at: string | null;
};

export type NotificationPage = {
  items: NotificationItem[];
  next_offset: number | null;
  next_cursor?: string | null;
  unread_count: number;
};

export type NotificationPreference = {
  event_type: string;
  channel: "in_app" | "email";
  enabled: boolean;
};

export type NotificationSettings = {
  preferences: NotificationPreference[];
  muted_space_ids: string[];
  default_reminder_time: string;
  time_zone: string;
};

export class NotificationsApiError extends Error {
  constructor(public readonly status: number) {
    super("Notifications or preferences could not be loaded or saved.");
  }
}

export async function listNotifications(
  accessToken: string,
  continuation: string | number = 0,
  limit = 50,
  signal?: AbortSignal,
): Promise<NotificationPage> {
  const params = new URLSearchParams();
  params.set(
    typeof continuation === "string" ? "cursor" : "offset",
    String(continuation),
  );
  params.set("limit", String(limit));
  const response = await authenticatedFetch(`/api/notifications?${params}`, {
    headers: { Authorization: `Bearer ${accessToken}` },
    signal,
  });
  if (!response.ok) throw new NotificationsApiError(response.status);
  return (await response.json()) as NotificationPage;
}

export async function setNotificationRead(
  accessToken: string,
  id: string,
  read: boolean,
): Promise<void> {
  const response = await authenticatedFetch(`/api/notifications/${id}/read`, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ read }),
  });
  if (!response.ok) throw new NotificationsApiError(response.status);
}

export async function markAllNotificationsRead(
  accessToken: string,
): Promise<void> {
  const response = await authenticatedFetch("/api/notifications/read-all", {
    method: "POST",
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) throw new NotificationsApiError(response.status);
}

export async function getNotificationSettings(
  accessToken: string,
): Promise<NotificationSettings> {
  const response = await authenticatedFetch("/api/notifications/preferences", {
    headers: { Authorization: `Bearer ${accessToken}` },
  });
  if (!response.ok) throw new NotificationsApiError(response.status);
  return (await response.json()) as NotificationSettings;
}

export async function saveNotificationSettings(
  accessToken: string,
  settings: NotificationSettings,
): Promise<NotificationSettings> {
  const response = await authenticatedFetch("/api/notifications/preferences", {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(settings),
  });
  if (!response.ok) throw new NotificationsApiError(response.status);
  return (await response.json()) as NotificationSettings;
}
