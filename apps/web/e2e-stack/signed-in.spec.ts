import { randomUUID } from "node:crypto";
import { expect, test, type Page } from "@playwright/test";

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

  await page.setViewportSize({ width: 1440, height: 900 });
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByLabel("Email address")).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Email address")).toBeVisible();
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
