import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("requests a sign-in link and confirms the account-neutral success state", async ({
  page,
}) => {
  let submittedEmail: unknown;
  await page.route("**/api/auth/magic-links", async (route) => {
    submittedEmail = route.request().postDataJSON().email;
    await route.fulfill({ status: 202, json: { message: "accepted" } });
  });

  await page.goto("/welcome");
  await page.getByLabel("Email address").fill("maya@example.com");
  await page.getByRole("button", { name: "Send sign-in link" }).click();

  await expect(page.getByRole("status")).toContainText(
    "a sign-in link is on its way",
  );
  expect(submittedEmail).toBe("maya@example.com");
});

test("shows a useful sign-in failure and allows another attempt", async ({
  page,
}) => {
  let requests = 0;
  await page.route("**/api/auth/magic-links", async (route) => {
    requests += 1;
    await route.fulfill({
      status: requests === 1 ? 503 : 202,
      json: { message: requests === 1 ? "unavailable" : "accepted" },
    });
  });

  await page.goto("/welcome");
  await page.getByLabel("Email address").fill("maya@example.com");
  await page.getByRole("button", { name: "Send sign-in link" }).click();
  await expect(page.getByRole("alert")).toBeVisible();
  await page.getByRole("button", { name: "Send sign-in link" }).click();
  await expect(page.getByRole("status")).toContainText(
    "a sign-in link is on its way",
  );
  expect(requests).toBe(2);
});

test("explains a temporary rate limit without suggesting the account is disabled", async ({
  page,
}) => {
  await page.route("**/api/auth/magic-links", (route) =>
    route.fulfill({
      status: 429,
      headers: { "Retry-After": "60" },
      contentType: "application/problem+json",
      body: JSON.stringify({
        title: "Too many requests",
        status: 429,
        code: "rate_limited",
      }),
    }),
  );
  await page.goto("/welcome");
  await page.getByLabel("Email address").fill("maya@example.com");
  await page.getByRole("button", { name: "Send sign-in link" }).click();

  await expect(page.getByRole("alert")).toContainText(
    "wait about a minute before trying again",
  );
  await expect(page.getByRole("alert")).toContainText(
    "This does not affect your account",
  );
});

test("restores the signed-in account after a browser reload", async ({
  page,
}) => {
  let refreshRequests = 0;
  await page.route("**/api/auth/magic-links/redeem", (route) =>
    route.fulfill({
      status: 200,
      json: {
        account: { id: "user-1", email: "maya@example.com" },
        tokens: {
          access_token: "access-before-reload",
          refresh_token: "refresh-before-reload",
        },
      },
    }),
  );
  await page.route("**/api/auth/token/refresh", (route) => {
    refreshRequests += 1;
    return route.fulfill({
      status: 200,
      json: {
        access_token: "access-after-reload",
        refresh_token: "refresh-after-reload",
      },
    });
  });
  await page.route("**/api/account/me", (route) =>
    route.fulfill({
      status: 200,
      json: { id: "user-1", email: "maya@example.com" },
    }),
  );
  await page.route("**/api/spaces", (route) =>
    route.fulfill({ status: 200, json: [] }),
  );

  await page.goto("/auth/complete#token=one-time-secret");
  await expect(
    page.getByText("maya@example.com", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  await page.reload();
  await expect(
    page.getByText("maya@example.com", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  expect(refreshRequests).toBe(1);
});

test("redeems a magic-link callback once after leaving the callback URL", async ({
  page,
}) => {
  const redemptionRequests: Array<{
    body: unknown;
    pathAtRequest: string;
  }> = [];
  await page.route("**/api/auth/magic-links/redeem", async (route) => {
    redemptionRequests.push({
      body: route.request().postDataJSON(),
      pathAtRequest: new URL(page.url()).pathname,
    });
    await route.fulfill({
      status: 200,
      json: {
        account: { id: "user-1", email: "maya@example.com" },
        tokens: {
          access_token: "access-from-link",
          refresh_token: "refresh-from-link",
        },
      },
    });
  });
  await page.route("**/api/account/me", (route) =>
    route.fulfill({
      status: 200,
      json: { id: "user-1", email: "maya@example.com" },
    }),
  );
  await page.route("**/api/spaces", (route) =>
    route.fulfill({ status: 200, json: [] }),
  );

  await page.goto("/auth/complete#token=one-time-secret");

  await expect(
    page.getByText("maya@example.com", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  await expect(page).toHaveURL(/\/$/);
  await page.waitForLoadState("networkidle");

  expect(redemptionRequests).toEqual([
    {
      body: { token: "one-time-secret" },
      pathAtRequest: "/",
    },
  ]);
});

test("does not redeem a callback with no token", async ({ page }) => {
  let redemptionRequests = 0;
  await page.route("**/api/auth/magic-links/redeem", async (route) => {
    redemptionRequests += 1;
    await route.fulfill({ status: 401, json: { message: "invalid token" } });
  });

  await page.goto("/auth/complete");

  await expect(page.getByRole("alert")).toContainText(
    "This sign-in link is missing its token",
  );
  await expect(page).toHaveURL(/\/$/);
  expect(redemptionRequests).toBe(0);
});
