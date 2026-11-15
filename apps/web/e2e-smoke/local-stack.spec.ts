import { expect, test } from "@playwright/test";

test("browser readiness is backed by the local API and PostgreSQL", async ({
  page,
}) => {
  const readiness = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/health/ready") &&
      response.request().method() === "GET",
  );

  await page.goto("/");
  const response = await readiness;
  expect(response.status()).toBe(200);
  await expect(page.getByRole("status")).toContainText("Service is available.");
});
