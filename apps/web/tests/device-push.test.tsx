// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { DevicePushSettings } from "../src/features/planner/components/DevicePushSettings";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(navigator, "serviceWorker");
});

test("prompts only when registering and saves independent reminder preferences", async () => {
  const requestPermission = vi.fn().mockResolvedValue("granted");
  const unsubscribe = vi.fn().mockResolvedValue(true);
  const endpoint = "https://fcm.googleapis.com/fcm/send/test";
  const subscribe = vi.fn().mockResolvedValue({
    endpoint,
    unsubscribe,
    toJSON: () => ({ keys: { p256dh: "public", auth: "secret" } }),
  });
  vi.stubGlobal("PushManager", class {});
  vi.stubGlobal("Notification", { requestPermission });
  Object.defineProperty(navigator, "serviceWorker", {
    configurable: true,
    value: {
      getRegistration: async () => ({
        active: {},
        pushManager: { getSubscription: async () => null, subscribe },
      }),
    },
  });
  const fetchMock = vi.fn(async (_url: string, init?: RequestInit) =>
    init?.method
      ? new Response(null, { status: 204 })
      : Response.json({ public_key: "BA" }),
  );
  vi.stubGlobal("fetch", fetchMock);

  render(<DevicePushSettings accessToken="test-token" />);
  const register = await screen.findByRole("button", {
    name: "Register this device",
  });
  expect(requestPermission).not.toHaveBeenCalled();
  fireEvent.click(register);
  expect(requestPermission).toHaveBeenCalledOnce();
  const calendar = await screen.findByRole("switch", {
    name: "Calendar reminders",
  });
  fireEvent.click(calendar);
  await waitFor(() =>
    expect((calendar as HTMLInputElement).checked).toBe(false),
  );
  const preferences = fetchMock.mock.calls
    .filter(([, init]) => init?.method === "PUT")
    .map(([, init]) => JSON.parse(String(init?.body)));
  expect(preferences).toMatchObject([
    { endpoint, calendar_reminders: true, task_reminders: true },
    { endpoint, calendar_reminders: false, task_reminders: true },
  ]);
  fireEvent.click(
    screen.getByRole("button", { name: "Disconnect this device" }),
  );
  await waitFor(() => expect(unsubscribe).toHaveBeenCalledOnce());
  expect(
    await screen.findByRole("button", { name: "Register this device" }),
  ).toBeTruthy();
});
