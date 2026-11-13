// @vitest-environment jsdom
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { App } from "../src/App";

const account = { id: "user-1", email: "maya@example.com" };
const authResponse = {
  account,
  tokens: { access_token: "access-secret", refresh_token: "refresh-secret" },
};

beforeEach(() => {
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
});

function setPath(path: string) {
  window.history.replaceState({}, "", path);
}

function successfulAuthFetch() {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url === "/api/health/ready") return new Response("", { status: 200 });
    if (url === "/api/auth/magic-links/redeem") {
      return Response.json(authResponse);
    }
    if (url === "/api/account/me") return Response.json(account);
    if (url === "/api/spaces") return Response.json([]);
    if (url === "/api/auth/session/current")
      return new Response(null, { status: 204 });
    if (url === "/api/auth/magic-links") {
      return Response.json({ message: "accepted" }, { status: 202 });
    }
    throw new Error(`Unexpected request: ${url}`);
  });
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  setPath("/");
});

test("requests a real magic link and shows an account-neutral confirmation", async () => {
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  fireEvent.click(screen.getByRole("link", { name: "Sign in" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Email address" }), {
    target: { value: "maya@example.com" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Send sign-in link" }));

  expect(
    await screen.findByText(/If that address can receive mail/),
  ).toBeTruthy();
  expect(fetchMock).toHaveBeenCalledWith("/api/auth/magic-links", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: "maya@example.com" }),
  });
});

test("redeems the URL fragment, removes it, loads the account, and keeps tokens in memory", async () => {
  setPath("/auth/complete#token=one-time-secret");
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);

  expect(await screen.findByText("Signed in as")).toBeTruthy();
  expect(window.location.hash).toBe("");
  expect(fetchMock).toHaveBeenCalledWith("/api/auth/magic-links/redeem", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ token: "one-time-secret" }),
  });
  expect(fetchMock).toHaveBeenCalledWith("/api/account/me", {
    headers: { Authorization: "Bearer access-secret" },
  });
  expect(await screen.findByText("maya@example.com")).toBeTruthy();
});

test("shows an invalid-link message when redemption fails", async () => {
  setPath("/auth/complete#token=expired-secret");
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      if (String(input) === "/api/health/ready") {
        return new Response("", { status: 200 });
      }
      return new Response("{}", { status: 401 });
    }),
  );
  render(<App />);

  expect((await screen.findByRole("alert")).textContent).toContain(
    "invalid, expired, or already used",
  );
  expect(
    screen.getByRole("button", { name: "Request a new link" }),
  ).toBeTruthy();
});

test("signs out by revoking the refresh session and clears the account", async () => {
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
  setPath("/auth/complete#token=one-time-secret");
  const fetchMock = successfulAuthFetch();
  vi.stubGlobal("fetch", fetchMock);
  render(<App />);
  await screen.findByText("Signed in as");

  fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

  await waitFor(() =>
    expect(
      screen.getByRole("heading", { name: "Your life, across Spaces." }),
    ).toBeTruthy(),
  );
  expect(fetchMock).toHaveBeenCalledWith("/api/auth/session/current", {
    method: "DELETE",
    headers: { Authorization: "Bearer refresh-secret" },
  });
  expect(
    await screen.findByRole("textbox", { name: "Email address" }),
  ).toBeTruthy();
});
