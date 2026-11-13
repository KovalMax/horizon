// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { App } from "../src/App";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

test("shows loading and database-backed readiness", async () => {
  let resolve!: (response: Response) => void;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      () =>
        new Promise<Response>((r) => {
          resolve = r;
        }),
    ),
  );
  render(<App />);
  expect(screen.getByRole("status").textContent).toContain("Checking");
  resolve(new Response("", { status: 200 }));
  expect((await screen.findByText("Service is available.")).textContent).toBe(
    "Service is available.",
  );
  expect(fetch).toHaveBeenCalledWith("/api/health/ready", expect.any(Object));
});

test("shows an unavailable state when readiness fails", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("", { status: 503 })),
  );
  render(<App />);
  expect((await screen.findByRole("alert")).textContent).toContain(
    "unavailable",
  );
});

test("shows an unavailable state on network errors", async () => {
  vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network")));
  render(<App />);
  expect((await screen.findByRole("alert")).textContent).toContain(
    "unavailable",
  );
});

test("navigates to a Space and creates a session-only Task", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("", { status: 200 })),
  );
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  render(<App />);

  fireEvent.click(
    screen.getAllByRole("button", { name: /Linden Street Home/ })[0]!,
  );
  expect(await screen.findByRole("heading", { name: "Calendar" })).toBeTruthy();
  fireEvent.click(screen.getByRole("link", { name: "To-dos" }));
  expect(await screen.findByRole("heading", { name: "To-dos" })).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "New Task" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), {
    target: { value: "Review the weekly plan" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Add Task" }));

  expect(await screen.findByText("Review the weekly plan")).toBeTruthy();
});
