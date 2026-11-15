// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { App } from "../src/App";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

test("signed-out visitors see real authentication without planner routes", () => {
  vi.stubGlobal("fetch", vi.fn());
  render(<App />);

  expect(
    screen.getByRole("heading", { name: "Your life, across Spaces." }),
  ).toBeTruthy();
  expect(
    screen.getByRole("button", { name: "Send sign-in link" }),
  ).toBeTruthy();
  expect(screen.queryByRole("navigation", { name: "Spaces" })).toBeNull();
  expect(fetch).not.toHaveBeenCalled();
});
