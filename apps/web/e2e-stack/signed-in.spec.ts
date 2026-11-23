import { randomUUID } from "node:crypto";
import { devices, expect, test, webkit, type Page } from "@playwright/test";

const origin = "https://horizon.localhost";
const mailpit = process.env.HORIZON_MAILPIT_URL ?? "http://mailpit:8025";
const fixtureEmail = () => `browser-${randomUUID()}@example.test`;

type Inbox = { messages?: { ID: string; Subject: string }[] };

async function getMailLink(
  page: Page,
  email: string,
  subject: string,
  path: string,
): Promise<string> {
  let messageId = "";
  await expect
    .poll(
      async () => {
        const response = await page.request.get(`${mailpit}/api/v1/search`, {
          params: { query: `to:${email}` },
        });
        if (!response.ok()) return false;
        const inbox: Inbox = await response.json();
        messageId =
          inbox.messages?.find((message) => message.Subject === subject)?.ID ??
          "";
        return Boolean(messageId);
      },
      { timeout: 15_000, message: `Mailpit received: ${subject}` },
    )
    .toBe(true);

  const message = await page.request.get(
    `${mailpit}/api/v1/message/${encodeURIComponent(messageId)}`,
  );
  expect(message.status()).toBe(200);
  const { Text }: { Text: string } = await message.json();
  const links = Text.match(/https:\/\/horizon\.localhost\/[^\s<>]+/g) ?? [];
  const link = links.find((url) => new URL(url).pathname === path);
  if (!link) throw new Error(`The message did not contain a link to ${path}`);
  return link;
}

async function openPrivateLink(page: Page, link: string) {
  if (new URL(link).origin !== origin) {
    throw new Error("The mail link did not target the isolated browser stack");
  }
  try {
    await page.goto(link);
  } catch {
    throw new Error("Could not open the private link");
  }
}

async function signIn(
  page: Page,
  email: string,
  options: { entry?: string; joinRequest?: boolean } = {},
) {
  if (options.entry) await openPrivateLink(page, options.entry);
  else await page.goto("/welcome");
  await page.getByLabel("Email address").fill(email);
  await page.getByRole("button", { name: "Send sign-in link" }).click();
  await expect(page.getByRole("status")).toContainText(
    "a sign-in link is on its way",
  );
  await openPrivateLink(
    page,
    await getMailLink(
      page,
      email,
      "Your Horizon sign-in link",
      "/auth/complete",
    ),
  );
  if (options.joinRequest) {
    await expect(
      page.getByRole("button", { name: "Request access" }),
    ).toBeVisible();
  } else {
    await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  }
  expect(Boolean(new URL(page.url()).hash)).toBe(false);
}

async function createSpace(page: Page, name: string) {
  await page.getByRole("button", { name: "New shared Space" }).click();
  const dialog = page.getByRole("dialog", { name: "New shared Space" });
  await dialog.getByRole("textbox", { name: "Space name" }).fill(name);
  await dialog.getByRole("button", { name: "Create Space" }).click();
  await expect(
    page.getByRole("button", { name: `${name} Owner` }),
  ).toBeVisible();
  return new URL(page.url()).pathname.replace(/\/calendar$/, "");
}

async function createList(page: Page, title: string) {
  await page.getByRole("textbox", { name: "List name" }).fill(title);
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: `${title} · 0 open` }),
  ).toBeVisible();
}

test("signs in from Mailpit, persists a mobile Task, and signs out", async ({
  page,
}) => {
  await page.goto("/welcome");
  await expect
    .poll(() =>
      page.evaluate(async () => {
        const response = await fetch("/api/health/ready");
        return response.status;
      }),
    )
    .toBe(200);
  await signIn(page, fixtureEmail());
  await createSpace(page, "Weekend planning");
  await page.getByRole("link", { name: "All Spaces", exact: true }).click();
  await expect(
    page.getByRole("link", { name: /Weekend planning.*Open Space/ }),
  ).toBeVisible();
  await page.waitForLoadState("networkidle");
  const requests: string[] = [];
  const collect = (request: { url(): string }) => {
    const url = new URL(request.url());
    if (url.origin === origin && url.pathname.startsWith("/api/")) {
      requests.push(url.pathname);
    }
  };
  page.on("request", collect);
  try {
    await page.reload({ waitUntil: "networkidle" });
    await expect(
      page.getByRole("heading", { name: "Your Spaces", exact: true, level: 1 }),
    ).toBeVisible();
  } finally {
    page.off("request", collect);
  }
  expect(
    requests.filter((path) => path === "/api/invitations/current"),
  ).toHaveLength(1);
  expect(requests).not.toContain("/api/join-requests/current");
  expect(requests).not.toContain("/api/spaces/deleted");
  expect(requests.filter((path) => path.startsWith("/api/spaces/"))).toEqual(
    [],
  );
  expect(
    requests.length,
    `All Spaces requests: ${requests.join(", ")}`,
  ).toBeLessThanOrEqual(9);
  await page
    .getByRole("link", { name: /Weekend planning.*Open Space/ })
    .click();
  await page.getByRole("link", { name: "To-dos", exact: true }).click();
  await createList(page, "Weekend jobs");

  await page.setViewportSize({ width: 320, height: 640 });
  const trigger = page.getByRole("button", { name: "New Task", exact: true });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "New Task" });
  const close = dialog.getByRole("button", { name: "Close dialog" });
  const add = dialog.getByRole("button", { name: "Add Task", exact: true });
  await expect(close).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(add).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(trigger).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );

  await trigger.click();
  await dialog.getByRole("textbox", { name: "Title" }).fill("Water the plants");
  const assignee = dialog.getByRole("combobox", { name: /Assignee/ });
  const owner = assignee.getByRole("option", { name: / · Owner$/ });
  await expect(owner).toBeAttached();
  const ownerId = await owner.getAttribute("value");
  if (!ownerId) throw new Error("Owner missing from the Task assignee list");
  await assignee.selectOption(ownerId);
  await add.click();
  const task = page.getByRole("button", {
    name: "Water the plants",
    exact: true,
  });
  await expect(task).toBeVisible();
  await page.reload();
  await task.click();
  await expect(
    page.getByRole("dialog", { name: "Water the plants" }),
  ).toContainText("No due date");
  await page.keyboard.press("Escape");
  await expect(task).toBeFocused();
  await task.click();
  await page.getByRole("button", { name: "Edit Task", exact: true }).click();
  await expect(
    page
      .getByRole("dialog", { name: "Edit Task" })
      .getByRole("combobox", { name: /Assignee/ }),
  ).toHaveValue(ownerId);
  await page.keyboard.press("Escape");

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByLabel("Email address")).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Email address")).toBeVisible();
});

test("confirms whether to cancel one occurrence or delete a Calendar series", async ({
  page,
}) => {
  await signIn(page, fixtureEmail());
  await createSpace(page, "Calendar planning");
  await page.getByRole("button", { name: "New event", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "New event" });
  await editor.getByRole("textbox", { name: "Title" }).fill("Weekly planning");
  await editor
    .getByRole("combobox", { name: "Repeats" })
    .selectOption({ label: "Every week" });
  await editor.getByRole("button", { name: "Add event" }).click();

  const today = page.getByRole("region", { name: "Space calendar" });
  const occurrence = today.getByRole("button", {
    name: "Weekly planning",
    exact: true,
  });
  const details = page.getByRole("dialog", { name: "Weekly planning" });
  await occurrence.click();
  const cancelOccurrence = details.getByRole("button", {
    name: "Cancel this occurrence",
  });
  await cancelOccurrence.click();
  const cancel = page.getByRole("dialog", { name: "Cancel this occurrence?" });
  await expect(cancel).toContainText("The rest of the series will stay");
  await page.keyboard.press("Escape");
  await expect(cancel).toBeHidden();
  await expect(cancelOccurrence).toBeFocused();
  await cancelOccurrence.click();
  await cancel
    .getByRole("button", { name: "Cancel occurrence", exact: true })
    .click();
  await expect(occurrence).toHaveCount(0);

  await page.reload();
  const remaining = page.getByRole("button", {
    name: "Weekly planning",
    exact: true,
  });
  await remaining.first().click();
  await details
    .getByRole("button", { name: "Delete series", exact: true })
    .click();
  const remove = page.getByRole("dialog", { name: "Delete this series?" });
  await expect(remove).toContainText("every occurrence");
  await remove
    .getByRole("button", { name: "Delete series", exact: true })
    .click();
  await expect(remaining).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Calendar", exact: true }),
  ).toBeFocused();
  await page.reload();
  await expect(today.getByText("Nothing planned")).toBeVisible();
  await expect(remaining).toHaveCount(0);

  await page.getByRole("button", { name: "New event", exact: true }).click();
  await editor.getByRole("textbox", { name: "Title" }).fill("One-time meeting");
  await editor.getByRole("button", { name: "Add event" }).click();
  const meeting = today.getByRole("button", {
    name: "One-time meeting",
    exact: true,
  });
  await meeting.click();
  await page
    .getByRole("dialog", { name: "One-time meeting" })
    .getByRole("button", { name: "Delete event", exact: true })
    .click();
  const single = page.getByRole("dialog", {
    name: "Delete this Calendar event?",
  });
  await expect(single).toContainText("This cannot be undone");
  await single
    .getByRole("button", { name: "Delete event", exact: true })
    .click();
  await expect(meeting).toHaveCount(0);
});

test("the mobile time-zone list does not block the reminder time", async ({
  page,
}) => {
  await signIn(page, fixtureEmail());
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  const timeZone = page.getByRole("combobox", { name: "Time zone" });
  const options = page.getByRole("listbox", { name: "Time zones" });
  const reminder = page.getByLabel("Default reminder time");
  await timeZone.click();
  await expect(options).toBeVisible();
  await reminder.click({ timeout: 2_000 });
  await expect(options).toBeHidden();
  await expect(reminder).toBeFocused();
  await timeZone.click();
  await expect(options).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(options).toBeHidden();
  await expect(timeZone).toBeFocused();
});

test("Saved places filters expose selection and count only the current Space", async ({
  page,
}) => {
  await signIn(page, fixtureEmail());
  const grove = await createSpace(page, "Grove");
  const pier = await createSpace(page, "Pier");
  const savePlace = async (name: string, category: string) => {
    await page.getByRole("button", { name: "Save a place" }).click();
    const dialog = page.getByRole("dialog", { name: "Save a place" });
    await dialog.getByRole("textbox", { name: "Place name" }).fill(name);
    await dialog
      .getByRole("textbox", { name: "Address" })
      .fill("12 Market Street");
    await dialog
      .getByRole("combobox", { name: "Category" })
      .selectOption({ label: category });
    await dialog
      .getByRole("button", { name: "Save place", exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name, exact: true }),
    ).toBeVisible();
  };
  await page.goto(`${pier}/places`);
  await savePlace("Harbour shop", "Groceries");
  await page.goto(`${grove}/places`);
  await savePlace("Orchard café", "Food & drink");
  await savePlace("Grove shop", "Groceries");

  const filters = page.getByRole("group", { name: "Place categories" });
  const groceries = filters.getByRole("button", { name: "Groceries (1)" });
  await expect(
    filters.getByRole("button", { name: "All (2)", pressed: true }),
  ).toBeVisible();
  await groceries.click();
  await expect(
    filters.getByRole("button", { name: "Groceries (1)", pressed: true }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Grove shop" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Orchard café" })).toHaveCount(
    0,
  );
  await expect(page.getByRole("heading", { name: "Harbour shop" })).toHaveCount(
    0,
  );

  await page.getByRole("button", { name: "Pier Owner" }).click();
  await page.getByRole("link", { name: "Places", exact: true }).click();
  await expect(
    filters.getByRole("button", { name: "All (1)", pressed: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Harbour shop" }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Grove shop" })).toHaveCount(
    0,
  );
});

test("keeps the filled join-link expiration inside its form on mobile WebKit", async () => {
  const browser = await webkit.launch();
  try {
    const page = await browser.newPage({
      ...devices["iPhone 13"],
      baseURL: origin,
      ignoreHTTPSErrors: true,
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    await signIn(page, fixtureEmail());
    const space = await createSpace(page, "Mobile planning");
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(`${space}/members`);
    const role = page.getByRole("combobox", { name: "Role after approval" });
    const expiration = page.getByLabel(/Expires at/);
    await expiration.fill("2035-12-24T21:45");

    for (const width of [390, 375, 361, 320]) {
      await page.setViewportSize({ width, height: 844 });
      const roleBox = await role.boundingBox();
      const expiryBox = await expiration.boundingBox();
      expect(roleBox).not.toBeNull();
      expect(expiryBox).not.toBeNull();
      expect(expiryBox!.y).toBeGreaterThanOrEqual(roleBox!.y + roleBox!.height);
      expect(expiryBox!.x + expiryBox!.width).toBeLessThanOrEqual(
        roleBox!.x + roleBox!.width + 1,
      );
      expect(expiryBox!.height).toBeGreaterThanOrEqual(43);
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
      ).toBe(width);
    }
    await expect(expiration).toHaveValue("2035-12-24T21:45");
    await expiration.fill("");
    await expect(expiration).toHaveValue("");
  } finally {
    await browser.close();
  }
});

test("keeps Settings time controls usable on mobile WebKit", async () => {
  const browser = await webkit.launch();
  try {
    const page = await browser.newPage({
      ...devices["iPhone 13"],
      baseURL: origin,
      ignoreHTTPSErrors: true,
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    await signIn(page, fixtureEmail());
    await page.getByRole("link", { name: "Settings", exact: true }).click();
    // The saved preferences populate the form asynchronously after navigation.
    await expect(
      page.getByRole("button", { name: "Save settings" }),
    ).toBeEnabled();

    const timeZone = page.getByRole("combobox", { name: "Time zone" });
    const options = page.getByRole("listbox", { name: "Time zones" });
    const reminder = page.getByLabel("Default reminder time");
    await page.setViewportSize({ width: 390, height: 844 });
    await reminder.fill("21:45");

    for (const width of [390, 375, 320]) {
      await page.setViewportSize({ width, height: 844 });
      await timeZone.tap();
      await expect(options).toBeVisible();
      const search = await timeZone.boundingBox();
      const list = await options.boundingBox();
      const time = await reminder.boundingBox();
      expect(Math.abs(time!.x - search!.x)).toBeLessThanOrEqual(1);
      expect(Math.abs(time!.width - search!.width)).toBeLessThanOrEqual(1);
      expect(Math.abs(time!.height - search!.height)).toBeLessThanOrEqual(1);
      expect(list!.y + list!.height).toBeLessThanOrEqual(time!.y);
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
      ).toBe(width);
      await reminder.tap();
      await expect(options).toBeHidden();
    }

    // iOS Safari can zoom into focused controls whose text is under 16px.
    await expect(timeZone).toHaveCSS("font-size", "16px");
    await expect(reminder).toHaveCSS("font-size", "16px");
    await expect(reminder).toHaveValue("21:45");
  } finally {
    await browser.close();
  }
});

test("an approved Editor writes shared content that an invited Viewer can only read", async ({
  page: owner,
  browser,
}) => {
  await signIn(owner, fixtureEmail());
  const title = "Household planning";
  const space = await createSpace(owner, title);
  await owner.getByRole("link", { name: "Members", exact: true }).click();

  await owner
    .getByRole("combobox", { name: "Role after approval" })
    .selectOption({ label: "Editor" });
  const linkCreated = owner.waitForResponse(
    (r) =>
      r.request().method() === "POST" &&
      new URL(r.url()).pathname.endsWith("/join-links"),
  );
  await owner.getByRole("button", { name: "Create join link" }).click();
  const response = await linkCreated;
  expect(response.status()).toBe(201);
  const { url }: { url: string } = await response.json();

  const editorContext = await browser.newContext({
    baseURL: origin,
    ignoreHTTPSErrors: true,
  });
  const viewerContext = await browser.newContext({
    baseURL: origin,
    ignoreHTTPSErrors: true,
  });
  try {
    const editor = await editorContext.newPage();
    const editorEmail = fixtureEmail();
    await signIn(editor, editorEmail, { entry: url, joinRequest: true });
    await editor.getByRole("button", { name: "Request access" }).click();
    await expect(editor.getByRole("status")).toContainText(
      "Your request is pending Owner approval.",
    );
    await owner.reload();
    const requests = owner.locator("section").filter({
      has: owner.getByRole("heading", { name: /^Join requests/ }),
    });
    await expect(
      requests.getByText(editorEmail, { exact: true }),
    ).toBeVisible();
    await requests
      .getByRole("button", { name: "Approve", exact: true })
      .click();
    await expect(
      owner.getByRole("combobox", { name: `Role for ${editorEmail}` }),
    ).toHaveValue("editor");
    await editor.goto(`${space}/tasks`);
    await expect(
      editor.getByRole("button", { name: `${title} Editor` }),
    ).toBeVisible();
    await createList(editor, "Shopping");
    await editor.getByRole("link", { name: "Members", exact: true }).click();
    await expect(
      editor.getByRole("textbox", { name: "Email address to invite" }),
    ).toHaveCount(0);

    const viewer = await viewerContext.newPage();
    const viewerEmail = fixtureEmail();
    await owner
      .getByRole("textbox", { name: "Email address to invite" })
      .fill(viewerEmail);
    await owner.getByRole("button", { name: /Invite$/ }).click();
    const invitation = await getMailLink(
      owner,
      viewerEmail,
      `You're invited to ${title} on Horizon`,
      "/welcome",
    );
    await signIn(viewer, viewerEmail, { entry: invitation });
    await viewer.getByRole("button", { name: "Accept", exact: true }).click();
    await expect(
      viewer.getByRole("button", { name: `${title} Viewer` }),
    ).toBeVisible();
    await viewer.goto(`${space}/tasks`);
    await expect(
      viewer.getByRole("heading", { name: "Shopping · 0 open" }),
    ).toBeVisible();
    await expect(viewer.getByRole("button", { name: "New Task" })).toHaveCount(
      0,
    );
    await expect(
      viewer.getByRole("textbox", { name: "List name" }),
    ).toHaveCount(0);
  } finally {
    await Promise.all([editorContext.close(), viewerContext.close()]);
  }
});
