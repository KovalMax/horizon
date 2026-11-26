import { useEffect, useRef, useState } from "react";
import {
  getNotificationSettings,
  listNotifications,
  markAllNotificationsRead as markAllNotificationsReadApi,
  NotificationsApiError,
  saveNotificationSettings,
  setNotificationRead,
  type NotificationItem,
} from "./notificationsApi";

const notificationCategories: Record<string, string> = {
  task_assignment: "Task assignments",
  membership_change: "Membership changes",
  event_change: "Event changes",
  join_request: "Join requests",
  join_request_result: "Join request results",
  due_reminder: "Due reminders",
};

const initialNotificationPreferences = Object.fromEntries(
  Object.values(notificationCategories).flatMap((category) => [
    [`${category}:In-app`, true],
    [`${category}:Email`, false],
  ]),
) as Record<string, boolean>;

export function useNotifications(
  accessToken: string,
  path: string,
  showToast: (message: string) => void,
) {
  const [reminderTime, setReminderTime] = useState("09:00");
  const [timeZone, setTimeZone] = useState(
    () => Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
  );
  const [notificationPrefs, setNotificationPrefs] = useState<
    Record<string, boolean>
  >(initialNotificationPreferences);
  const [notificationPrefsStatus, setNotificationPrefsStatus] = useState<
    "idle" | "loading" | "ready" | "saving" | "error"
  >("idle");
  const [notificationPrefsError, setNotificationPrefsError] = useState("");
  const [mutedSpaces, setMutedSpaces] = useState<string[]>([]);
  const [unread, setUnread] = useState(0);
  const [notifications, setNotifications] = useState<NotificationItem[]>([]);
  const [notificationsStatus, setNotificationsStatus] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [notificationsError, setNotificationsError] = useState("");
  const [notificationActionError, setNotificationActionError] = useState("");
  const [notificationsContinuation, setNotificationsContinuation] = useState<
    string | number | null
  >(null);
  const loadMoreRequest = useRef<AbortController | null>(null);
  const [notificationsReload, setNotificationsReload] = useState(0);
  const [loadingMoreNotifications, setLoadingMoreNotifications] =
    useState(false);

  useEffect(() => {
    let active = true;
    setNotificationPrefsStatus("loading");
    setNotificationPrefsError("");
    getNotificationSettings(accessToken)
      .then((settings) => {
        if (!active) return;
        const preferences = { ...initialNotificationPreferences };
        for (const preference of settings.preferences) {
          const category = notificationCategories[preference.event_type];
          if (!category) continue;
          const channel = preference.channel === "in_app" ? "In-app" : "Email";
          preferences[`${category}:${channel}`] = preference.enabled;
        }
        setNotificationPrefs(preferences);
        setMutedSpaces(settings.muted_space_ids);
        setReminderTime(settings.default_reminder_time);
        setTimeZone(settings.time_zone);
        setNotificationPrefsStatus("ready");
      })
      .catch(() => {
        if (!active) return;
        setNotificationPrefsStatus("error");
        setNotificationPrefsError(
          "Notification preferences couldn’t be loaded. Retry or check your connection.",
        );
      });
    return () => {
      active = false;
    };
  }, [accessToken]);

  useEffect(() => {
    let active = true;
    listNotifications(accessToken, 0, 1)
      .then((page) => {
        if (active) setUnread(page.unread_count);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [accessToken]);

  useEffect(() => {
    if (path !== "/notifications") {
      setNotifications([]);
      setNotificationsContinuation(null);
      setNotificationsStatus("idle");
      return;
    }
    let active = true;
    setNotifications([]);
    setNotificationsContinuation(null);
    setLoadingMoreNotifications(false);
    setNotificationsStatus("loading");
    setNotificationsError("");
    listNotifications(accessToken)
      .then((page) => {
        if (!active) return;
        setNotifications(page.items);
        setNotificationsContinuation(page.next_cursor ?? page.next_offset);
        setUnread(page.unread_count);
        setNotificationsStatus("ready");
      })
      .catch(() => {
        if (!active) return;
        setNotificationsStatus("error");
        setNotificationsError(
          "Notifications couldn’t be loaded. Check your connection and retry.",
        );
      });
    return () => {
      active = false;
      loadMoreRequest.current?.abort();
      loadMoreRequest.current = null;
    };
  }, [accessToken, path, notificationsReload]);

  const markNotificationRead = async (
    notification: NotificationItem,
    read: boolean,
  ) => {
    setNotificationActionError("");
    try {
      await setNotificationRead(accessToken, notification.id, read);
      if (read && !notification.read_at)
        setUnread((count) => Math.max(0, count - 1));
      if (!read && notification.read_at) setUnread((count) => count + 1);
      const readAt = read
        ? (notification.read_at ?? new Date().toISOString())
        : null;
      setNotifications((all) =>
        all.map((item) =>
          item.id === notification.id ? { ...item, read_at: readAt } : item,
        ),
      );
      return true;
    } catch {
      setNotificationActionError(
        "That notification couldn’t be updated. Retry your action.",
      );
      return false;
    }
  };

  const markAllNotificationsRead = async () => {
    setNotificationActionError("");
    try {
      await markAllNotificationsReadApi(accessToken);
      const readAt = new Date().toISOString();
      setNotifications((all) =>
        all.map((item) => ({ ...item, read_at: item.read_at ?? readAt })),
      );
      setUnread(0);
      showToast("All notifications marked as read.");
    } catch {
      setNotificationActionError(
        "Notifications couldn’t be marked as read. Retry your action.",
      );
    }
  };

  const loadMoreNotifications = async () => {
    if (notificationsContinuation === null || loadMoreRequest.current) return;
    const request = new AbortController();
    loadMoreRequest.current = request;
    setNotificationActionError("");
    setLoadingMoreNotifications(true);
    try {
      const page = await listNotifications(
        accessToken,
        notificationsContinuation,
        50,
        request.signal,
      );
      if (request.signal.aborted) return;
      setNotifications((all) => [...all, ...page.items]);
      setNotificationsContinuation(page.next_cursor ?? page.next_offset);
      setUnread(page.unread_count);
    } catch {
      if (request.signal.aborted) return;
      setNotificationActionError(
        "More notifications couldn’t be loaded. Retry your action.",
      );
    } finally {
      if (loadMoreRequest.current === request) {
        loadMoreRequest.current = null;
        setLoadingMoreNotifications(false);
      }
    }
  };

  const savePreferences = async (
    mutedSpaceIds: string[] = mutedSpaces,
    successMessage = "Settings saved.",
  ): Promise<boolean> => {
    setNotificationPrefsStatus("saving");
    setNotificationPrefsError("");
    const preferences = Object.entries(notificationCategories).flatMap(
      ([eventType, category]) => [
        {
          event_type: eventType,
          channel: "in_app" as const,
          enabled: notificationPrefs[`${category}:In-app`] ?? true,
        },
        {
          event_type: eventType,
          channel: "email" as const,
          enabled: notificationPrefs[`${category}:Email`] ?? false,
        },
      ],
    );
    try {
      const settings = await saveNotificationSettings(accessToken, {
        preferences,
        muted_space_ids: mutedSpaceIds,
        default_reminder_time: reminderTime,
        time_zone: timeZone,
      });
      setMutedSpaces(settings.muted_space_ids);
      setNotificationPrefsStatus("ready");
      showToast(successMessage);
      return true;
    } catch (error) {
      setNotificationPrefsStatus("error");
      setNotificationPrefsError(
        error instanceof NotificationsApiError && error.status === 400
          ? "Check the notification settings and try again."
          : "Settings couldn’t be saved. Retry your action.",
      );
      return false;
    }
  };

  const setSpaceMuted = async (
    spaceId: string,
    muted: boolean,
  ): Promise<boolean> => {
    if (notificationPrefsStatus !== "ready") return false;
    const previousMutedSpaces = mutedSpaces;
    const nextMutedSpaces = muted
      ? [...new Set([...mutedSpaces, spaceId])]
      : mutedSpaces.filter((id) => id !== spaceId);
    setMutedSpaces(nextMutedSpaces);
    const saved = await savePreferences(
      nextMutedSpaces,
      muted ? "Space muted." : "Space unmuted.",
    );
    if (!saved) {
      setMutedSpaces(previousMutedSpaces);
      setNotificationPrefsStatus("ready");
    }
    return saved;
  };

  return {
    reminderTime,
    setReminderTime,
    timeZone,
    setTimeZone,
    notificationPrefs,
    setNotificationPrefs,
    notificationPrefsStatus,
    notificationPrefsError,
    mutedSpaces,
    setMutedSpaces,
    setSpaceMuted,
    unread,
    setUnread,
    notifications,
    notificationsStatus,
    notificationsError,
    notificationActionError,
    hasMoreNotifications: notificationsContinuation !== null,
    loadingMoreNotifications,
    retryNotifications: () => setNotificationsReload((current) => current + 1),
    markNotificationRead,
    markAllNotificationsRead,
    loadMoreNotifications,
    savePreferences,
  };
}
