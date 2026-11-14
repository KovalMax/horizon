import { expect, test } from "@playwright/test";

test("requests a sign-in link and confirms the neutral success message", async ({
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

test("creates a Task from the Space to-do list", async ({ page }) => {
  await page.goto("/spaces/home/tasks");
  await page.getByRole("button", { name: "New Task" }).click();

  await expect(page.getByRole("heading", { name: "New Task" })).toBeVisible();
  await page.getByLabel("Title").fill("Pack rain jackets");
  await page.getByLabel("Due date").fill("2030-06-15");
  await expect(page.getByLabel("1 day before")).toBeChecked();
  await expect(page.getByLabel("1 hour before")).toBeChecked();
  await page.getByRole("button", { name: "Add Task" }).click();

  const createdTask = page
    .locator(".task-row")
    .filter({ has: page.getByRole("button", { name: "Pack rain jackets" }) });
  await expect(createdTask).toBeVisible();
  await expect(page.getByText("Task added (prototype only).")).toBeVisible();
});
