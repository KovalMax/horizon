import { randomUUID } from "node:crypto";
import { devices, expect, test, webkit, type Page } from "@playwright/test";

const origin = "https://horizon.localhost";
const mailpit = process.env.HORIZON_MAILPIT_URL ?? "http://mailpit:8025";
const fixtureEmail = () => `browser-${randomUUID()}@example.test`;

type Inbox = { messages?: { ID: string; Subject: string }[] };

async function getMail(
  page: Page,
  email: string,
  subject: string,
): Promise<{ Text: string; HTML: string }> {
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
  return message.json();
}

async function getMailText(page: Page, email: string, subject: string) {
  return (await getMail(page, email, subject)).Text;
}

function findMailLink(text: string, path: string): string {
  const links = text.match(/https:\/\/horizon\.localhost\/[^\s<>]+/g) ?? [];
  const link = links.find((url) => new URL(url).pathname === path);
  if (!link) throw new Error(`The message did not contain a link to ${path}`);
  return link;
}

async function getMailLink(
  page: Page,
  email: string,
  subject: string,
  path: string,
) {
  return findMailLink(await getMailText(page, email, subject), path);
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
  const upcoming = page.getByRole("region", { name: "Today & coming up" });
  const emptyTimeline = upcoming.getByRole("heading", {
    name: "Nothing scheduled in the next two weeks",
  });
  const undated = page.getByRole("region", { name: "No due date" });
  const spaces = page.getByRole("region", { name: "Your Spaces" });
  await expect(emptyTimeline).toBeVisible();
  await expect(undated).toHaveCount(0);
  await createSpace(page, "Weekend planning");
  await page.getByRole("link", { name: "All Spaces", exact: true }).click();
  const weekend = spaces.getByRole("link", { name: "Weekend planning" });
  await expect(weekend).toBeVisible();
  await expect(weekend).toHaveText("Weekend planning");
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
      page.getByRole("heading", { name: "Your plans", exact: true, level: 1 }),
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
  await weekend.click();
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
  const [createdTask] = await Promise.all([
    page.waitForResponse(
      (response) =>
        /^\/api\/spaces\/[^/]+\/tasks$/.test(
          new URL(response.url()).pathname,
        ) &&
        response.request().method() === "POST" &&
        response.status() === 201,
    ),
    add.click(),
  ]);
  const { id: taskId } = (await createdTask.json()) as { id: string };
  const taskRoute = new URL(page.url()).pathname;
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

  await page.goto("/");
  await expect(emptyTimeline).toBeVisible();
  await expect(
    undated.getByRole("button", { name: "Water the plants", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("main").getByRole("region")).toHaveText([
    /Today & coming up/,
    /No due date/,
    /Your Spaces/,
  ]);
  await upcoming.getByRole("button", { name: "Events", exact: true }).click();
  await expect(undated).toHaveCount(0);

  await page.goto(`${taskRoute}?reminder=${taskId}`);
  await expect(
    page.getByRole("dialog", { name: "Water the plants" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");

  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "More", exact: true }).click();
  await page
    .getByRole("dialog", { name: "More" })
    .getByRole("link", { name: "Settings", exact: true })
    .click();
  await page
    .getByRole("region", { name: "Account" })
    .getByRole("button", { name: "Sign out" })
    .click();
  await expect(page).toHaveURL(/\/welcome$/);
  await expect(page.getByLabel("Email address")).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Email address")).toBeVisible();
});

test("rejects a nonexistent mail domain before sending a sign-in email", async ({
  page,
}) => {
  await page.goto("/welcome");
  await page
    .getByLabel("Email address")
    .fill(`${randomUUID()}@horizon.invalid`);
  await page.getByRole("button", { name: "Send sign-in link" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "This email domain doesn’t receive mail.",
  );
});

test("signs in with the emailed code without opening the magic link", async ({
  page,
}) => {
  const email = fixtureEmail();
  await page.goto("/welcome");
  await page.getByLabel("Email address").fill(email);
  await page.getByRole("button", { name: "Send sign-in link" }).click();
  const input = page.getByRole("textbox", { name: "Sign-in code" });
  await expect(input).toBeVisible();
  const { Text: text, HTML: html } = await getMail(
    page,
    email,
    "Your Horizon sign-in link",
  );
  const code = text.match(/^\s*(\d{8})\s*$/m)?.[1];
  if (!code) throw new Error("The sign-in email did not contain a code");
  const visualGroups = html.match(
    /<span style="padding-right:0.45em">(\d{4})<\/span><span>(\d{4})<\/span>/,
  );
  expect(
    visualGroups !== null && visualGroups[1] + visualGroups[2] === code,
    "HTML displays two adjacent groups with no clipboard separator",
  ).toBe(true);
  const link = findMailLink(text, "/auth/complete");
  const htmlMatchesPlainText = await page.evaluate(
    ({ html, link, code }) => {
      const email = new DOMParser().parseFromString(html, "text/html");
      const action = email.querySelector("a");
      return (
        action?.textContent === "Sign in to Horizon" &&
        action.getAttribute("href") === link &&
        email.querySelector("img")?.getAttribute("src") ===
          new URL("/horizon-touch-180.png", link).href &&
        email.querySelector("code")?.textContent === code
      );
    },
    { html, link, code },
  );
  expect(
    htmlMatchesPlainText,
    "Email uses the app icon and preserves the plain-text sign-in details",
  ).toBe(true);
  await page.reload();
  await input.fill(code).catch(() => {
    throw new Error("Could not enter the sign-in code");
  });
  await page.getByRole("button", { name: "Sign in with code" }).click();
  await expect(page.getByRole("heading", { name: "Your plans" })).toBeVisible();
  const fragment = new URL(link).hash;
  const token = new URLSearchParams(fragment.slice(1)).get("token");
  const replayStatus = await page.evaluate(async (token) => {
    const response = await fetch("/api/auth/magic-links/redeem", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ token }),
    });
    return response.status;
  }, token);
  expect(replayStatus).toBe(401);
});

test("the installable app serves only a public fallback offline", async ({
  page,
}) => {
  await signIn(page, fixtureEmail());
  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute(
    "href",
    "/manifest.webmanifest",
  );
  const { status, contentType, manifest } = await page.evaluate(async () => {
    const response = await fetch("/manifest.webmanifest");
    return {
      status: response.status,
      contentType: response.headers.get("content-type"),
      manifest: await response.json(),
    };
  });
  expect(status).toBe(200);
  expect(contentType).toContain("application/manifest+json");
  expect(manifest).toMatchObject({
    name: "Horizon",
    start_url: "/",
    display: "standalone",
    icons: expect.arrayContaining([
      expect.objectContaining({ sizes: "192x192" }),
      expect.objectContaining({ sizes: "512x512" }),
    ]),
  });
  const iconPaths = [
    ...manifest.icons.map((icon: { src: string }) => icon.src),
    "/horizon-touch-180.png",
  ];
  expect(
    await page.evaluate(
      (paths) =>
        Promise.all(
          paths.map(async (path) => {
            const image = new Image();
            image.src = path;
            await image.decode();
            return `${image.naturalWidth}x${image.naturalHeight}`;
          }),
        ),
      iconPaths,
    ),
  ).toEqual(["192x192", "512x512", "180x180"]);
  await page.waitForFunction(
    () => Boolean(navigator.serviceWorker.controller),
    undefined,
    { timeout: 10_000 },
  );
  const cachedPaths = await page.evaluate(async () => {
    const requests = await Promise.all(
      (await caches.keys()).map(async (key) => (await caches.open(key)).keys()),
    );
    return requests.flat().map((request) => new URL(request.url).pathname);
  });
  expect(cachedPaths).toEqual(["/offline.html"]);

  await page.context().setOffline(true);
  try {
    await page.goto("/settings");
    await expect(
      page.getByRole("heading", { name: "You’re offline" }),
    ).toBeVisible();
    await expect(page.getByRole("heading", { name: "Settings" })).toHaveCount(
      0,
    );
    // Check callback bypass without booting the auth app offline in this tab.
    const callbackPage = await page.context().newPage();
    try {
      await expect(
        callbackPage.goto("/auth/complete?offline-check=1", { timeout: 5_000 }),
      ).rejects.toThrow("net::ERR_INTERNET_DISCONNECTED");
    } finally {
      await callbackPage.close();
    }
  } finally {
    await page.context().setOffline(false);
  }
  await page.getByRole("link", { name: "Try again" }).click();
  await expect(page.getByRole("heading", { name: "Your plans" })).toBeVisible({
    timeout: 15_000,
  });
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  const update = page.getByRole("button", { name: "Update app" });
  await Promise.all([
    page.waitForEvent("framenavigated", {
      predicate: (frame) => frame === page.mainFrame(),
    }),
    update.click(),
  ]);
  await expect(update).toBeEnabled();
  await expect(
    page.getByRole("region", { name: "Account" }).getByRole("button", {
      name: "Sign out",
    }),
  ).toBeVisible();
});

test("the Space filter border stays inside its scroll area on hover", async ({
  page,
}) => {
  await signIn(page, fixtureEmail());
  const filters = page.getByRole("group", { name: "Filter by Space" });
  const personal = filters.getByRole("button", { name: "Personal" });
  for (const width of [1023, 390, 320]) {
    await page.setViewportSize({ width, height: 750 });
    await personal.hover();
    expect(
      await personal.evaluate(async (pill) => {
        await Promise.all(
          pill.getAnimations().map((animation) => animation.finished),
        );
        return (
          pill.getBoundingClientRect().top >=
          pill.parentElement!.getBoundingClientRect().top
        );
      }),
    ).toBe(true);
  }
});

test("confirms whether to cancel one occurrence or delete a Calendar series", async ({
  page,
}) => {
  await signIn(page, fixtureEmail());
  await createSpace(page, "Calendar planning");
  await page.getByRole("button", { name: "New event", exact: true }).click();
  const editor = page.getByRole("dialog", { name: "New event" });
  await editor.getByRole("textbox", { name: "Title" }).fill("Weekly planning");
  const reminders = editor.getByRole("group", { name: "Remind me" });
  const firstReminder = reminders.getByRole("combobox", {
    name: "Reminder 1",
    exact: true,
  });
  await expect(firstReminder).toHaveValue("");
  await firstReminder.selectOption("60");
  await expect(
    reminders.getByRole("combobox", { name: "Reminder 2", exact: true }),
  ).toHaveValue("");
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
  await expect(meeting).toBeVisible();
  await page.getByRole("link", { name: "All Spaces", exact: true }).click();
  const upcoming = page.getByRole("region", { name: "Today & coming up" });
  await expect(
    upcoming.getByRole("button", { name: "One-time meeting", exact: true }),
  ).toBeVisible();
  await expect(
    upcoming.getByRole("heading", {
      name: "Nothing scheduled in the next two weeks",
    }),
  ).toHaveCount(0);
  await page.goBack();
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

test("reviews and saves a Maps place with appended notes on mobile", async ({
  page,
}) => {
  await signIn(page, fixtureEmail());
  const space = await createSpace(page, "Restaurant plans");
  await page.goto(`${space}/places`);
  await page.setViewportSize({ width: 320, height: 640 });
  await page.getByRole("button", { name: "Save a place" }).click();
  const dialog = page.getByRole("dialog", { name: "Save a place" });
  const name = dialog.getByLabel("Place name");
  await name.fill("Dinner spot");
  await dialog.getByText("Import from Google Maps", { exact: true }).click();
  const mapsLink = dialog.getByLabel("Google Maps link");
  await mapsLink.fill("https://www.google.com/maps/dir/?api=1");
  const importDetails = dialog.getByRole("button", { name: "Import details" });
  await importDetails.click();
  await expect(dialog.getByRole("alert")).toContainText(
    "supported Google Maps",
  );
  await expect(name).toHaveValue("Dinner spot");
  await mapsLink.fill(
    "https://maps.google.com?q=Pepella+georgisches+Restaurant+und+Bar,+Augustastra%C3%9Fe+30,+40477+D%C3%BCsseldorf-Stadtbezirk+1,+Germany&ftid=0x47b8c988bbf31717:0x744e25ef6964ea62",
  );
  await importDetails.click();
  await expect(dialog.getByLabel("Address")).toHaveValue(
    "Augustastraße 30, 40477 Düsseldorf-Stadtbezirk 1, Germany",
  );
  await expect(name).toHaveValue("Dinner spot");
  await dialog.getByRole("button", { name: /Use imported name/ }).click();
  await expect(name).toHaveValue("Pepella georgisches Restaurant und Bar");
  const note = dialog.getByLabel("Note (optional)");
  await note.fill("Book a table.");
  await dialog.getByText("Add opening hours, cuisine, or website").click();
  await dialog.getByLabel("Opening hours").fill("Tue–Fri 17:00–23:00");
  await dialog.getByLabel("Cuisine").fill("Georgian");
  await dialog
    .getByLabel("Website", { exact: true })
    .fill("https://www.restaurantpepella.com/");
  await dialog.getByRole("button", { name: "Add to note" }).click();
  await expect(note).toHaveValue(
    "Book a table.\n\nHours: Tue–Fri 17:00–23:00\nCuisine: Georgian\nWebsite: https://www.restaurantpepella.com/",
  );
  expect(await dialog.evaluate((el) => el.scrollWidth - el.clientWidth)).toBe(
    0,
  );
  await dialog.getByRole("button", { name: "Save place", exact: true }).click();
  await expect(
    page.getByRole("heading", {
      name: "Pepella georgisches Restaurant und Bar",
    }),
  ).toBeVisible();
  await expect(page.getByText(/Cuisine: Georgian/)).toBeVisible();
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
  const browser = await webkit.launch({ args: [] });
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
  const browser = await webkit.launch({ args: [] });
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
    const reminderControl = page.locator(".settings-reminder-control", {
      has: reminder,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await reminder.fill("21:45");

    for (const width of [390, 375, 320]) {
      await page.setViewportSize({ width, height: 844 });
      await timeZone.tap();
      await expect(options).toBeVisible();
      const search = await timeZone.boundingBox();
      const list = await options.boundingBox();
      const time = await reminderControl.boundingBox();
      expect(Math.abs(time!.x - search!.x)).toBeLessThanOrEqual(1);
      expect(Math.abs(time!.width - search!.width)).toBeLessThanOrEqual(1);
      expect(Math.abs(time!.height - search!.height)).toBeLessThanOrEqual(1);
      const editor = await reminder.boundingBox();
      const topInset = editor!.y - time!.y;
      const bottomInset = time!.y + time!.height - editor!.y - editor!.height;
      expect(topInset).toBeGreaterThan(1);
      expect(Math.abs(topInset - bottomInset)).toBeLessThanOrEqual(1);
      expect(editor!.x).toBeGreaterThanOrEqual(time!.x);
      expect(editor!.x + editor!.width).toBeLessThanOrEqual(
        time!.x + time!.width,
      );
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

test("mobile WebKit keeps planner date and time fields inside their sheets", async () => {
  const browser = await webkit.launch({ args: [] });
  try {
    const page = await browser.newPage({
      ...devices["iPhone 13"],
      baseURL: origin,
      ignoreHTTPSErrors: true,
    });
    await page.setViewportSize({ width: 1440, height: 900 });
    await signIn(page, fixtureEmail());
    const space = await createSpace(page, "Mobile planning");
    await page.goto(`${space}/tasks`);
    await createList(page, "Mobile tasks");
    await page.goto(`${space}/calendar`);

    const expectSheetFits = async (name: string) => {
      const sheet = page.getByRole("dialog", { name });
      for (const width of [390, 320]) {
        await page.setViewportSize({ width, height: 640 });
        expect(
          await sheet.evaluate(
            (element) => element.scrollWidth - element.clientWidth,
          ),
        ).toBe(0);
        expect(
          await sheet
            .locator('input:is([type="date"], [type="time"])')
            .evaluateAll((inputs) =>
              inputs
                .filter((input) => input.getClientRects().length)
                .every((input) => {
                  const control = input.getBoundingClientRect();
                  const field = input.closest("label")!.getBoundingClientRect();
                  return (
                    control.left >= field.left - 1 &&
                    control.right <= field.right + 1
                  );
                }),
            ),
        ).toBe(true);
      }
    };

    await page.setViewportSize({ width: 390, height: 640 });
    await page.getByRole("button", { name: "New event", exact: true }).tap();
    const event = page.getByRole("dialog", { name: "New event" });
    await expectSheetFits("New event");
    const starts = (await event.getByLabel("Starts").boundingBox())!;
    const ends = (await event.getByLabel("Ends").boundingBox())!;
    expect(Math.abs(starts.y - ends.y)).toBeLessThanOrEqual(1);
    expect(starts.x + starts.width).toBeLessThan(ends.x);
    await event.getByLabel("Starts").fill("17:15");
    const allDay = event.getByRole("switch", { name: "All-day event" });
    const insets = await allDay.evaluate((toggle) => {
      const row = toggle.closest("label")!;
      const bounds = row.getBoundingClientRect();
      const text = row.querySelector("span")!.getBoundingClientRect();
      return {
        text: text.left - bounds.left,
        toggle: bounds.right - toggle.getBoundingClientRect().right,
      };
    });
    expect(insets.text).toBeGreaterThanOrEqual(12);
    expect(insets.toggle).toBeGreaterThanOrEqual(12);
    expect(Math.abs(insets.text - insets.toggle)).toBeLessThanOrEqual(1);
    await allDay.tap();
    await expect(allDay).toBeChecked();
    await expect(event.getByLabel("Starts", { exact: true })).toBeHidden();
    await expect(event.getByLabel("Ends", { exact: true })).toBeHidden();
    const date = event.getByLabel("Date", { exact: true });
    await expect(date).toBeVisible();
    await date.fill("2026-12-02");
    await expectSheetFits("New event");
    await allDay.tap();
    await expect(event.getByLabel("Starts")).toBeVisible();
    await expect(event.getByLabel("Starts")).toHaveValue("17:15");
    await expect(date).toHaveValue("2026-12-02");
    await event.getByText("Set an end date (optional)").tap();
    await event.getByLabel("Repeat through").fill("2026-12-02");
    await expectSheetFits("New event");
    await event.getByRole("button", { name: "Close dialog" }).tap();

    const more = page.getByRole("button", { name: "More", exact: true });
    await more.tap();
    const drawer = page.getByRole("dialog", { name: "More" });
    const close = drawer.getByRole("button", { name: "Close More menu" });
    await expect(close).toBeFocused();
    expect((await close.boundingBox())!.height).toBeGreaterThanOrEqual(44);
    await close.tap();
    await expect(drawer).toBeHidden();
    await expect(more).toBeFocused();
    await more.tap();
    await drawer.getByRole("link", { name: "Space settings" }).tap();
    await expect(drawer).toBeHidden();

    await page.getByRole("button", { name: "To-dos", exact: true }).tap();
    await page.getByRole("button", { name: "New Task", exact: true }).tap();
    const task = page.getByRole("dialog", { name: "New Task" });
    await expect(task.getByLabel("Due time (optional)")).toBeDisabled();
    await task.getByLabel("Due date (optional)").fill("2026-12-02");
    await task.getByLabel("Due time (optional)").fill("21:45");
    await expectSheetFits("New Task");
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
