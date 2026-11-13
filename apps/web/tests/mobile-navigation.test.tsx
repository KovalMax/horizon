// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { App } from "../src/App";

function setPath(path: string) {
  window.history.replaceState({}, "", path);
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  setPath("/");
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: 1024,
  });
});

test("mobile Space switcher shows current Space details and preserves the open section", async () => {
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: 390,
  });
  setPath("/spaces/home/tasks");
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("", { status: 200 })),
  );
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  render(<App />);

  fireEvent.click(
    screen.getByRole("button", {
      name: "Switch Space. Current: Linden Street Home",
    }),
  );
  expect(screen.getByRole("dialog", { name: "Switch Space" })).toBeTruthy();
  expect(
    screen.getByRole("button", {
      name: /Linden Street Home Owner · 3 members Current/,
    }),
  ).toBeTruthy();

  fireEvent.click(
    screen.getByRole("button", {
      name: /Lisbon long weekend Editor · 4 members/,
    }),
  );

  expect(await screen.findByRole("heading", { name: "To-dos" })).toBeTruthy();
  expect(document.querySelector(".space-title")?.textContent).toContain(
    "Lisbon long weekend",
  );
});

test("mobile More exposes product navigation without preview tools", () => {
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: 390,
  });
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("", { status: 200 })),
  );
  render(<App />);

  fireEvent.click(screen.getByRole("button", { name: "More" }));

  expect(screen.getByRole("dialog", { name: "More" })).toBeTruthy();
  expect(screen.getByRole("link", { name: /Members & sharing/ })).toBeTruthy();
  expect(
    screen.getByRole("link", { name: /Notification settings/ }),
  ).toBeTruthy();
  expect(
    screen.queryByText(/Review guide|Prototype controls|Simulate offline/),
  ).toBeNull();
  expect(screen.queryByRole("dialog", { name: "Switch Space" })).toBeNull();
});
