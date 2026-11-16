// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
  waitFor,
} from "@testing-library/react";
import { App } from "../src/App";

function open(path: string) {
  window.history.replaceState({}, "", path);
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("", { status: 200 })),
  );
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  render(<App />);
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.history.replaceState({}, "", "/");
});

test("signed-in users list API Spaces and create a persisted shared Space", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const createdSpace = {
    id: "9beadf74-4172-4e91-b1ac-e447ed836cd6",
    name: "Family plans",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  const existingSpace = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "Shared household",
    kind: "shared",
    role: "editor",
    member_count: 3,
  };
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
      if (url === "/api/spaces" && !init?.method)
        return Response.json([existingSpace]);
      if (url === "/api/spaces" && init?.method === "POST")
        return Response.json(createdSpace, { status: 201 });
      if (url === "/api/join-requests/current") return Response.json([]);
      if (url.includes("/join-links")) return Response.json([]);
      if (url.includes("/join-requests")) return Response.json([]);
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  await screen.findByText("Signed in as");
  expect(
    (await screen.findAllByText("Shared household")).length,
  ).toBeGreaterThan(0);
  expect(screen.queryByText("Yoga class")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "New shared Space" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Space name" }), {
    target: { value: "Family plans" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Create Space" }));

  expect((await screen.findAllByText("Family plans")).length).toBeGreaterThan(
    0,
  );
  expect(screen.getAllByText("Owner").length).toBeGreaterThan(0);
  expect(window.location.pathname).toBe(`/spaces/${createdSpace.id}/calendar`);
  await waitFor(() =>
    expect(fetchMock).toHaveBeenCalledWith("/api/spaces", {
      method: "POST",
      headers: {
        Authorization: "Bearer access-secret",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ name: "Family plans" }),
    }),
  );
});

test("Space listing errors can be retried", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  let spaceRequests = 0;
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
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
      if (url === "/api/spaces") {
        spaceRequests += 1;
        return spaceRequests === 1
          ? new Response("{}", { status: 503 })
          : Response.json([
              {
                id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
                name: "Shared household",
                kind: "shared",
                role: "owner",
                member_count: 1,
              },
            ]);
      }
      if (url === "/api/join-requests/current") return Response.json([]);
      if (url.includes("/join-links")) return Response.json([]);
      if (url.includes("/join-requests")) return Response.json([]);
      throw new Error(`Unexpected request: ${url}`);
    }),
  );
  render(<App />);

  expect(await screen.findByText(/Spaces couldn’t be loaded/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  expect(
    (await screen.findAllByText("Shared household")).length,
  ).toBeGreaterThan(0);
  expect(spaceRequests).toBe(2);
});

test("signed-in Owners can edit/delete Tasks and confirm cascading List deletion", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  const list = {
    id: "d3e8b514-1785-4dbf-94ca-f2d7cbddff10",
    space_id: space.id,
    space_name: space.name,
    space_kind: space.kind,
    name: "Weekend prep",
  };
  const task = {
    id: "7bb52132-4f78-4e09-bf5e-f40be0d3ea4c",
    space_id: space.id,
    space_name: space.name,
    space_kind: space.kind,
    list_id: list.id,
    list_name: list.name,
    title: "Buy groceries",
    due_date: null,
    due_time: null,
    done: false,
    assignee_id: null,
    assignee_email: null,
  };
  const remainingTask = {
    ...task,
    id: "0a9520a9-8e18-40fb-9caf-2d499687db6b",
    title: "Pack snacks",
  };
  let currentTask = task;
  let currentList = list;
  let deletedTask = false;
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
      if (url === "/api/spaces") return Response.json([space]);
      if (url.startsWith("/api/events?"))
        return Response.json({ items: [], next_offset: null });
      if (url === "/api/places?offset=0")
        return Response.json({ items: [], next_offset: null });
      if (url.startsWith("/api/tasks?"))
        return Response.json({
          lists: [currentList],
          tasks: deletedTask ? [remainingTask] : [currentTask, remainingTask],
          lists_next_offset: null,
          tasks_next_offset: null,
        });
      if (url === `/api/spaces/${space.id}/members`)
        return Response.json([
          { user_id: "user-1", email: "maya@example.com", role: "owner" },
        ]);
      if (url === `/api/spaces/${space.id}/invitations`)
        return Response.json([]);
      if (url === "/api/invitations/current") return Response.json([]);
      if (url === `/api/tasks/${task.id}` && init?.method === "PUT") {
        const body = JSON.parse(String(init.body));
        currentTask = {
          ...currentTask,
          title: body.title,
          list_name: body.list_name,
        };
        return Response.json(currentTask);
      }
      if (url === `/api/tasks/${task.id}` && init?.method === "DELETE") {
        deletedTask = true;
        return new Response(null, { status: 204 });
      }
      if (url === `/api/lists/${list.id}` && init?.method === "PUT") {
        const body = JSON.parse(String(init.body));
        currentList = { ...currentList, name: body.name };
        currentTask = { ...currentTask, list_name: body.name };
        return Response.json(currentList);
      }
      if (url === `/api/lists/${list.id}` && init?.method === "DELETE")
        return new Response(null, { status: 204 });
      if (url === "/api/join-requests/current") return Response.json([]);
      if (url.includes("/join-links")) return Response.json([]);
      if (url.includes("/join-requests")) return Response.json([]);
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(screen.getAllByRole("button", { name: "To-dos" })[0]!);
  fireEvent.click(await screen.findByRole("button", { name: "Buy groceries" }));
  fireEvent.click(screen.getByRole("button", { name: "Edit Task" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), {
    target: { value: "Buy groceries and fruit" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save Task changes" }));
  expect(await screen.findByText("Buy groceries and fruit")).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/tasks/${task.id}`,
    expect.objectContaining({ method: "PUT" }),
  );

  fireEvent.click(
    screen.getByRole("button", { name: "Buy groceries and fruit" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Delete Task" }));
  expect(confirm).toHaveBeenCalledWith(
    "Delete this Task? This cannot be undone.",
  );
  await waitFor(() =>
    expect(fetchMock).toHaveBeenCalledWith(
      `/api/tasks/${task.id}`,
      expect.objectContaining({ method: "DELETE" }),
    ),
  );

  fireEvent.click(screen.getByRole("button", { name: "Rename Weekend prep" }));
  fireEvent.change(
    within(screen.getByRole("dialog")).getByRole("textbox", {
      name: "List name",
    }),
    {
      target: { value: "Errands" },
    },
  );
  fireEvent.click(screen.getByRole("button", { name: "Save list name" }));
  expect(await screen.findByText(/Errands · 1 open/)).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Delete Errands" }));
  expect(confirm).toHaveBeenCalledWith(
    "Delete “Errands” and all Tasks in it? This cannot be undone.",
  );
  await waitFor(() =>
    expect(fetchMock).toHaveBeenCalledWith(
      `/api/lists/${list.id}`,
      expect.objectContaining({ method: "DELETE" }),
    ),
  );
});

test("signed-in Calendar loads persisted events and creates one in the selected Space", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  let savedEvent: Record<string, unknown> | null = null;
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
      if (url === "/api/spaces") return Response.json([space]);
      if (url.startsWith("/api/events?") && !init?.method)
        return Response.json({ items: [], next_offset: null });
      if (url === `/api/spaces/${space.id}/events` && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        savedEvent = {
          id: "ad572d7b-c33e-4df9-a5eb-29a01c99e76e",
          space_id: space.id,
          space_name: space.name,
          space_kind: space.kind,
          title: body.title,
          starts_at: body.starts_at,
          ends_at: body.ends_at,
          time_zone: body.time_zone,
          all_day: body.all_day,
          location: body.location,
          notes: body.notes,
          recurrence: body.recurrence,
        };
        return Response.json(savedEvent, { status: 201 });
      }
      if (url === `/api/events/${savedEvent?.id}` && init?.method === "PUT") {
        const body = JSON.parse(String(init.body));
        savedEvent = { ...savedEvent, ...body };
        return Response.json(savedEvent);
      }
      if (url === `/api/events/${savedEvent?.id}` && init?.method === "DELETE")
        return new Response(null, { status: 204 });
      if (url === "/api/join-requests/current") return Response.json([]);
      if (url.includes("/join-links")) return Response.json([]);
      if (url.includes("/join-requests")) return Response.json([]);
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: "New event" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), {
    target: { value: "Persisted dinner" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add event" }));

  expect(await screen.findByText("Persisted dinner")).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/events`,
    expect.objectContaining({
      method: "POST",
      headers: expect.objectContaining({
        Authorization: "Bearer access-secret",
        "Content-Type": "application/json",
      }),
    }),
  );

  fireEvent.click(screen.getByRole("button", { name: "Persisted dinner" }));
  fireEvent.click(screen.getByRole("button", { name: "Edit event" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), {
    target: { value: "Updated dinner" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save event" }));
  expect(await screen.findByText("Updated dinner")).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/events/${savedEvent?.id}`,
    expect.objectContaining({ method: "PUT" }),
  );

  fireEvent.click(screen.getByRole("button", { name: "Updated dinner" }));
  fireEvent.click(screen.getByRole("button", { name: "Delete event" }));
  await waitFor(() =>
    expect(screen.queryByRole("button", { name: "Updated dinner" })).toBeNull(),
  );
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/events/${savedEvent?.id}`,
    expect.objectContaining({ method: "DELETE" }),
  );
});

test("signed-in Members can cancel one expanded recurring occurrence", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  const now = new Date();
  const occurrenceDate = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  const recurringEvent = {
    id: "ad572d7b-c33e-4df9-a5eb-29a01c99e76e",
    space_id: space.id,
    space_name: space.name,
    space_kind: space.kind,
    title: "Recurring dinner",
    starts_at: new Date(`${occurrenceDate}T18:00:00`).toISOString(),
    ends_at: new Date(`${occurrenceDate}T19:00:00`).toISOString(),
    time_zone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    all_day: false,
    location: null,
    notes: null,
    recurrence: "daily",
    recurrence_ends_on: null,
    occurrence_date: occurrenceDate,
    is_override: false,
  };
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
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
    if (url === "/api/spaces") return Response.json([space]);
    if (url.startsWith("/api/events?") || url.startsWith("/api/events/range?"))
      return Response.json({ items: [recurringEvent], next_offset: null });
    if (url.startsWith("/api/tasks?"))
      return Response.json({
        lists: [],
        tasks: [],
        lists_next_offset: null,
        tasks_next_offset: null,
      });
    if (url === "/api/places?offset=0")
      return Response.json({ items: [], next_offset: null });
    if (url === `/api/spaces/${space.id}/members`)
      return Response.json([
        { user_id: "user-1", email: "maya@example.com", role: "owner" },
      ]);
    if (
      url === `/api/spaces/${space.id}/invitations` ||
      url === "/api/invitations/current"
    )
      return Response.json([]);
    if (
      url === `/api/events/${recurringEvent.id}/occurrences/${occurrenceDate}`
    )
      return new Response(null, { status: 204 });
    if (url === "/api/join-requests/current") return Response.json([]);
    if (url.includes("/join-links")) return Response.json([]);
    if (url.includes("/join-requests")) return Response.json([]);
    throw new Error(`Unexpected request: ${url}`);
  });
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(
    await screen.findByRole("button", { name: "Recurring dinner" }),
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Cancel this occurrence" }),
  );
  await waitFor(() =>
    expect(
      screen.queryByRole("button", { name: "Recurring dinner" }),
    ).toBeNull(),
  );
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/events/${recurringEvent.id}/occurrences/${occurrenceDate}`,
    expect.objectContaining({ method: "DELETE" }),
  );
});

test("signed-in To-dos load persisted lists and Tasks and save completion changes", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  let persistedTasks: Array<Record<string, unknown>> = [];
  let lists: Array<Record<string, unknown>> = [];
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
      if (url === "/api/spaces") return Response.json([space]);
      if (url === `/api/spaces/${space.id}/members`)
        return Response.json([
          {
            user_id: "user-1",
            email: "maya@example.com",
            role: "owner",
          },
          {
            user_id: "member-2",
            email: "ruth@example.test",
            role: "viewer",
          },
        ]);
      if (url === `/api/spaces/${space.id}/invitations`)
        return Response.json([]);
      if (url === "/api/invitations/current") return Response.json([]);
      if (url.startsWith("/api/events?"))
        return Response.json({ items: [], next_offset: null });
      if (url.startsWith("/api/tasks?"))
        return Response.json({
          lists,
          tasks: persistedTasks,
          lists_next_offset: null,
          tasks_next_offset: null,
        });
      if (url === `/api/spaces/${space.id}/lists` && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const list = {
          id: "d2f4431d-1caf-4208-a0ee-1d5ca9d281d0",
          space_id: space.id,
          space_name: space.name,
          space_kind: space.kind,
          name: body.name,
        };
        lists = [...lists, list];
        return Response.json(list, { status: 201 });
      }
      if (url === `/api/spaces/${space.id}/tasks` && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const task = {
          id: "22c39af2-71e4-4cbc-9c7c-5afcf258e5e5",
          space_id: space.id,
          space_name: space.name,
          space_kind: space.kind,
          list_id: "d2f4431d-1caf-4208-a0ee-1d5ca9d281d0",
          title: body.title,
          list_name: body.list_name,
          due_date: body.due_date,
          due_time: body.due_time,
          done: false,
          assignee_id: body.assignee_id,
          assignee_email:
            body.assignee_id === "member-2" ? "ruth@example.test" : null,
        };
        persistedTasks = [...persistedTasks, task];
        return Response.json(task, { status: 201 });
      }
      if (
        url === `/api/tasks/${persistedTasks[0]?.id}/completion` &&
        init?.method === "PUT"
      ) {
        const body = JSON.parse(String(init.body));
        persistedTasks = [{ ...persistedTasks[0], ...body }];
        return Response.json(persistedTasks[0]);
      }
      if (url === "/api/join-requests/current") return Response.json([]);
      if (url.includes("/join-links")) return Response.json([]);
      if (url.includes("/join-requests")) return Response.json([]);
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(
    within(spacesNavigation).getByRole("link", { name: "To-dos" }),
  );
  fireEvent.change(screen.getByRole("textbox", { name: "List name" }), {
    target: { value: "Weekend prep" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add" }));
  expect(
    await screen.findByText("To-do list saved to this Space."),
  ).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "New Task" }));
  fireEvent.change(
    await screen.findByRole("combobox", { name: "Assigned to" }),
    {
      target: { value: "member-2" },
    },
  );
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), {
    target: { value: "Buy groceries" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add Task" }));
  expect(
    await screen.findByRole("button", { name: "Buy groceries" }),
  ).toBeTruthy();
  expect(await screen.findByText("ruth@example.test")).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/tasks`,
    expect.objectContaining({
      method: "POST",
      body: expect.stringContaining('"assignee_id":"member-2"'),
    }),
  );

  fireEvent.click(
    screen.getByRole("button", { name: "Complete Buy groceries" }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Mark Buy groceries open" }),
    ).toBeTruthy(),
  );
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/tasks/${persistedTasks[0]?.id}/completion`,
    expect.objectContaining({ method: "PUT" }),
  );
});

test("signed-in Members load and save Places in their selected Space", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  let savedPlaces: Array<Record<string, unknown>> = [];
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
      if (url === "/api/spaces") return Response.json([space]);
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
        return Response.json({ items: savedPlaces, next_offset: null });
      if (url === `/api/spaces/${space.id}/places` && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const place = {
          id: "974ba0b9-1f8c-49fb-a0a8-a7774f4ad81c",
          space_id: space.id,
          space_name: space.name,
          space_kind: space.kind,
          name: body.name,
          category: body.category,
          address: body.address,
          note: body.note,
        };
        savedPlaces = [place];
        return Response.json(place, { status: 201 });
      }
      if (url === "/api/join-requests/current") return Response.json([]);
      if (url.includes("/join-links")) return Response.json([]);
      if (url.includes("/join-requests")) return Response.json([]);
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(
    within(spacesNavigation).getByRole("link", { name: "Places" }),
  );
  expect(
    await screen.findByRole("heading", { name: "No saved places yet" }),
  ).toBeTruthy();
  expect(
    screen.getAllByRole("button", { name: "Save a place", exact: true }),
  ).toHaveLength(1);
  fireEvent.click(screen.getAllByRole("button", { name: "Save a place" })[0]!);
  fireEvent.change(screen.getByRole("textbox", { name: "Place name" }), {
    target: { value: "Neighborhood bakery" },
  });
  fireEvent.change(screen.getByRole("textbox", { name: "Address" }), {
    target: { value: "12 Linden Street" },
  });
  fireEvent.change(screen.getByRole("textbox", { name: "Note (optional)" }), {
    target: { value: "Best sourdough before noon" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save place" }));

  expect(
    await screen.findByRole("heading", { name: "Neighborhood bakery" }),
  ).toBeTruthy();
  expect(screen.getByText("12 Linden Street")).toBeTruthy();
  expect(screen.getByText("Best sourdough before noon")).toBeTruthy();
  expect(
    screen.getAllByRole("button", { name: "Save a place", exact: true }),
  ).toHaveLength(1);
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/places`,
    expect.objectContaining({
      method: "POST",
      headers: expect.objectContaining({
        Authorization: "Bearer access-secret",
      }),
      body: JSON.stringify({
        name: "Neighborhood bakery",
        category: "Food & drink",
        address: "12 Linden Street",
        note: "Best sourdough before noon",
      }),
    }),
  );
});

test("signed-in Owners see real Members and send Viewer invitations", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  const member = {
    user_id: "user-1",
    email: "maya@example.com",
    role: "owner",
  };
  let invitations: Array<Record<string, unknown>> = [];
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
      if (url === "/api/spaces") return Response.json([space]);
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
      if (url === `/api/spaces/${space.id}/members`)
        return Response.json([member]);
      if (
        url === `/api/spaces/${space.id}/invitations` &&
        init?.method === "POST"
      ) {
        const body = JSON.parse(String(init.body));
        const invitation = {
          id: "invite-1",
          space_id: space.id,
          space_name: space.name,
          email: body.email,
          role: "viewer",
          expires_at: "2026-11-20T12:00:00Z",
        };
        invitations = [invitation];
        return Response.json(invitation, { status: 201 });
      }
      if (url === `/api/spaces/${space.id}/invitations`)
        return Response.json(invitations);
      if (url === "/api/invitations/current") return Response.json([]);
      if (url === "/api/join-requests/current") return Response.json([]);
      if (url.includes("/join-links")) return Response.json([]);
      if (url.includes("/join-requests")) return Response.json([]);
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(
    within(spacesNavigation).getByRole("link", { name: "Members" }),
  );
  expect(
    (await screen.findAllByText("maya@example.com")).length,
  ).toBeGreaterThan(0);
  fireEvent.change(
    screen.getByRole("textbox", { name: "Email address to invite" }),
    {
      target: { value: "jonas@example.com" },
    },
  );
  fireEvent.click(screen.getByRole("button", { name: "＋ Invite" }));

  expect(await screen.findByText("jonas@example.com")).toBeTruthy();
  expect(screen.getByText("Viewer · Pending")).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/invitations`,
    expect.objectContaining({
      method: "POST",
      headers: expect.objectContaining({
        Authorization: "Bearer access-secret",
      }),
    }),
  );
});

test("Space Owners create reusable links and approve join requests", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  const request = {
    id: "4191eeca-67b8-4167-8c3a-c57f63fb2bde",
    link_id: "3f2cab12-8d0f-4f9a-8901-4337b73232ea",
    space_id: space.id,
    space_name: space.name,
    requester_id: "2beafda9-fb94-4084-86ca-d670a77c9306",
    requester_email: "jonas@example.test",
    role: "editor",
    status: "pending",
    requested_at: "2026-11-13T12:00:00Z",
    decided_at: null,
  };
  const link = {
    id: request.link_id,
    space_id: space.id,
    space_name: space.name,
    role: "editor",
    created_at: "2026-11-13T12:00:00Z",
    expires_at: null,
    revoked_at: null,
    url: `http://localhost:5173/welcome?join=${request.link_id}`,
  };
  let requests: (typeof request)[] = [request];
  let links: (typeof link)[] = [];
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
      if (url === "/api/spaces") return Response.json([space]);
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
      if (url === `/api/spaces/${space.id}/members`)
        return Response.json([
          { user_id: "user-1", email: "maya@example.com", role: "owner" },
        ]);
      if (
        url === `/api/spaces/${space.id}/invitations` ||
        url === "/api/invitations/current"
      )
        return Response.json([]);
      if (url === "/api/join-requests/current") return Response.json([]);
      if (
        url === `/api/spaces/${space.id}/join-links` &&
        init?.method === "POST"
      ) {
        links = [link];
        return Response.json(link, { status: 201 });
      }
      if (url === `/api/spaces/${space.id}/join-links`)
        return Response.json(links);
      if (url === `/api/spaces/${space.id}/join-requests`)
        return Response.json(requests);
      if (
        url === `/api/spaces/${space.id}/join-requests/${request.id}/decision`
      ) {
        requests = [];
        return Response.json({
          ...request,
          status: "approved",
          decided_at: new Date().toISOString(),
        });
      }
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(
    within(spacesNavigation).getByRole("link", { name: "Members" }),
  );
  fireEvent.change(
    screen.getByRole("combobox", { name: "Role after approval" }),
    {
      target: { value: "editor" },
    },
  );
  fireEvent.click(screen.getByRole("button", { name: "Create join link" }));
  expect(await screen.findByText(link.url)).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/join-links`,
    expect.objectContaining({
      method: "POST",
      body: JSON.stringify({ role: "editor", expires_at: null }),
    }),
  );

  fireEvent.click(screen.getByRole("button", { name: "Approve" }));
  expect(await screen.findByText("No pending join requests.")).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/join-requests/${request.id}/decision`,
    expect.objectContaining({
      method: "POST",
      body: JSON.stringify({ decision: "approved" }),
    }),
  );
});

test("Space Owners transfer a shared Space to an existing Member", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "API household",
    kind: "shared",
    role: "owner",
    member_count: 2,
  };
  let ownerRole = "owner";
  let memberRole = "viewer";
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
      if (url === "/api/spaces")
        return Response.json([{ ...space, role: ownerRole }]);
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
      if (url === `/api/spaces/${space.id}/members`)
        return Response.json([
          { user_id: "user-1", email: "maya@example.com", role: ownerRole },
          { user_id: "user-2", email: "ruth@example.test", role: memberRole },
        ]);
      if (
        url === `/api/spaces/${space.id}/invitations` ||
        url === "/api/invitations/current"
      )
        return Response.json([]);
      if (url === "/api/spaces/deleted" || url === "/api/join-requests/current")
        return Response.json([]);
      if (
        url === `/api/spaces/${space.id}/join-links` ||
        url === `/api/spaces/${space.id}/join-requests`
      )
        return Response.json([]);
      if (url === `/api/spaces/${space.id}/owner` && init?.method === "PATCH") {
        ownerRole = "editor";
        memberRole = "owner";
        return Response.json({
          space_id: space.id,
          previous_owner_id: "user-1",
          new_owner_id: "user-2",
        });
      }
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /API household/,
    }),
  );
  fireEvent.click(
    await within(spacesNavigation).findByRole("link", {
      name: "Space settings",
    }),
  );
  fireEvent.change(await screen.findByRole("combobox", { name: "New Owner" }), {
    target: { value: "user-2" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Transfer ownership" }));

  expect(
    await screen.findByText(
      "Ownership transferred. You now have Editor access.",
    ),
  ).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/owner`,
    expect.objectContaining({
      method: "PATCH",
      body: JSON.stringify({ new_owner_id: "user-2" }),
    }),
  );
  expect(window.confirm).toHaveBeenCalledWith(
    "Transfer ownership of API household to ruth@example.test? You will become an Editor.",
  );
});

test("Owners delete a Space and restore it from Recently deleted", async () => {
  window.history.replaceState({}, "", "/auth/complete#token=one-time-secret");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const space = {
    id: "17982261-8323-42bf-94bb-6fba68f6b2d1",
    name: "Lost itinerary",
    kind: "shared",
    role: "owner",
    member_count: 1,
  };
  const deletedSpace = {
    id: space.id,
    name: space.name,
    kind: "shared",
    deleted_at: "2026-11-13T12:00:00Z",
    restore_until: "2026-11-20T12:00:00Z",
  };
  let deleted = false;
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
      if (url === "/api/spaces") return Response.json(deleted ? [] : [space]);
      if (url === "/api/spaces/deleted")
        return Response.json(deleted ? [deletedSpace] : []);
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
      if (url === `/api/spaces/${space.id}/members`)
        return Response.json([
          { user_id: "user-1", email: "maya@example.com", role: "owner" },
        ]);
      if (
        url === `/api/spaces/${space.id}/invitations` ||
        url === "/api/invitations/current"
      )
        return Response.json([]);
      if (url === "/api/join-requests/current") return Response.json([]);
      if (
        url === `/api/spaces/${space.id}/join-links` ||
        url === `/api/spaces/${space.id}/join-requests`
      )
        return Response.json([]);
      if (url === `/api/spaces/${space.id}` && init?.method === "DELETE") {
        deleted = true;
        return Response.json(deletedSpace);
      }
      if (
        url === `/api/spaces/${space.id}/restore` &&
        init?.method === "POST"
      ) {
        deleted = false;
        return Response.json(space);
      }
      throw new Error(`Unexpected request: ${url}`);
    },
  );
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  const spacesNavigation = await screen.findByRole("navigation", {
    name: "Spaces",
  });
  fireEvent.click(
    await within(spacesNavigation).findByRole("button", {
      name: /Lost itinerary/,
    }),
  );
  fireEvent.click(
    await within(spacesNavigation).findByRole("link", {
      name: "Space settings",
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Delete Space" }));
  expect(window.confirm).toHaveBeenCalledWith(
    "Delete Lost itinerary? You can restore it for seven days. After that, it and its content will be permanently deleted.",
  );

  fireEvent.click(await screen.findByRole("link", { name: "Settings" }));
  expect(await screen.findByText(/Restorable until/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Restore" }));

  expect(await screen.findByRole("heading", { name: "Calendar" })).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}`,
    expect.objectContaining({ method: "DELETE" }),
  );
  expect(fetchMock).toHaveBeenCalledWith(
    `/api/spaces/${space.id}/restore`,
    expect.objectContaining({ method: "POST" }),
  );
});
