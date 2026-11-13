// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { App } from "../src/App";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

test("overview explains when all Spaces are filtered out", () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("", { status: 200 })),
  );
  render(<App />);

  for (const name of [
    "Maya's Space",
    "Linden Street Home",
    "Lisbon long weekend",
  ]) {
    const filter = screen
      .getAllByRole("button", { name: new RegExp(name) })
      .find((button) => button.hasAttribute("aria-pressed"));
    fireEvent.click(filter!);
  }

  expect(screen.getByText("All Spaces are filtered out")).toBeTruthy();
});
