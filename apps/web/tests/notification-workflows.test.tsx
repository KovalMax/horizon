// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { App } from "../src/App";

const notification = {
  id: "fcd0ddad-704b-43da-a661-61b36af72699",
  space_id: "1332f3d5-ea66-4134-b2f2-4d4698a72310",
  space_name: "Linden Street Home",
  event_type: "task_assignment",
  title: "Task assigned to you",
  body: "Renew home insurance",
  destination: "tasks",
  created_at: "2026-11-14T09:00:00Z",
  read_at: null,
};

const categories = [
  "task_assignment",
  "membership_change",
  "event_change",
  "join_request",
  "join_request_result",
  "due_reminder",
];

function defaultSettings() {
  return {
    preferences: categories.flatMap((event_type) => [
      { event_type, channel: "in_app", enabled: true },
      { event_type, channel: "email", enabled: false },
    ]),
    muted_space_ids: [],
    default_reminder_time: "09:00",
    time_zone: "UTC",
  };
}

async function setup(path: string) {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  let readAt: string | null = null;
  let unreadCount = 1;
  const fetchMock = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/health/ready") return new Response("", { status: 200 });
      if (url === "/api/auth/magic-links/redeem")
        return Response.json({
          account: { id: "user-1", email: "maya@example.com" },
          tokens: {
            access_token: "access-secret",
            refresh_token: "refresh-secret",
          },
        });
      if (url === "/api/account/me")
        return Response.json({ id: "user-1", email: "maya@example.com" });
      if (url === "/api/spaces") return Response.json([]);
      if (url === "/api/spaces/deleted") return Response.json([]);
      if (
        url === "/api/invitations/current" ||
        url === "/api/join-requests/current"
      )
        return Response.json([]);
      if (
        url === "/api/notifications/preferences" &&
        (!init || init.method === "GET")
      )
        return Response.json(defaultSettings());
      if (url === "/api/notifications/preferences" && init?.method === "PUT")
        return Response.json(JSON.parse(String(init.body)));
      if (url.startsWith("/api/notifications?"))
        return Response.json({
          items: [{ ...notification, read_at: readAt }],
          next_offset: null,
          unread_count: unreadCount,
        });
      if (
        url === `/api/notifications/${notification.id}/read` &&
        init?.method === "PUT"
      ) {
        readAt = JSON.parse(String(init.body)).read
          ? "2026-11-14T10:00:00Z"
          : null;
        unreadCount = readAt ? 0 : 1;
        return new Response(null, { status: 204 });
      }
      if (url === "/api/notifications/read-all" && init?.method === "POST") {
        readAt = "2026-11-14T10:00:00Z";
        unreadCount = 0;
        return new Response(null, { status: 204 });
      }
      if (url.startsWith("/api/events?"))
        return Response.json({ items: [], next_offset: null });
      if (url.startsWith("/api/tasks?"))
        return Response.json({
          lists: [],
          tasks: [],
          lists_next_offset: null,
          tasks_next_offset: null,
        });
      if (url.startsWith("/api/places?"))
        return Response.json({ items: [], next_offset: null });
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);
  await screen.findByText("Signed in as");
  fireEvent.click(
    path === "/notifications"
      ? screen.getAllByRole("link", { name: /Notifications/ })[0]!
      : screen.getByRole("link", { name: "Settings" }),
  );
  return fetchMock;
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.sessionStorage.clear();
  window.localStorage.clear();
  window.history.replaceState({}, "", "/");
});

test("inbox displays persisted items and supports explicit read state", async () => {
  const fetchMock = await setup("/notifications");

  expect(
    await screen.findByRole("heading", { name: "Task assigned to you" }),
  ).toBeTruthy();
  expect(screen.getByText("Renew home insurance")).toBeTruthy();
  expect(screen.getByText("Linden Street Home")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Mark as read" }));

  await waitFor(() => {
    expect(fetchMock).toHaveBeenCalledWith(
      `/api/notifications/${notification.id}/read`,
      expect.objectContaining({
        method: "PUT",
        body: JSON.stringify({ read: true }),
      }),
    );
  });
  expect(
    await screen.findByRole("button", { name: "Mark as unread" }),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Mark as unread" }));
  expect(
    await screen.findByRole("button", { name: "Mark as read" }),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Mark all read" }));
  await waitFor(() =>
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/notifications/read-all",
      expect.objectContaining({ method: "POST" }),
    ),
  );
});

test("Save settings persists notification channels and profile preferences together", async () => {
  const fetchMock = await setup("/settings");
  expect(
    await screen.findByRole("heading", { name: "Notification channels" }),
  ).toBeTruthy();
  const assignmentInApp = screen.getAllByRole("checkbox", {
    name: "In-app",
  })[0]!;
  fireEvent.click(assignmentInApp);
  const timeZone = screen.getByRole("combobox", { name: "Time zone" });
  fireEvent.focus(timeZone);
  fireEvent.change(timeZone, { target: { value: "Berlin" } });
  fireEvent.click(screen.getByRole("option", { name: /Berlin.*GMT/ }));
  fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

  await waitFor(() => {
    const saveCall = fetchMock.mock.calls.find(
      ([url, init]) =>
        String(url) === "/api/notifications/preferences" &&
        init?.method === "PUT",
    );
    expect(saveCall).toBeTruthy();
    const saved = JSON.parse(String(saveCall?.[1]?.body));
    expect(saved.preferences).toHaveLength(12);
    expect(saved.time_zone).toBe("Europe/Berlin");
    expect(saved.default_reminder_time).toBe("09:00");
    expect(
      saved.preferences.find(
        (item: { event_type: string; channel: string }) =>
          item.event_type === "task_assignment" && item.channel === "in_app",
      ).enabled,
    ).toBe(false);
  });
});
