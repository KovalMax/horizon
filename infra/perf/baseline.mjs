import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { readFile, writeFile } from "node:fs/promises";
import { availableParallelism, totalmem } from "node:os";
import { performance } from "node:perf_hooks";
import { setTimeout as delay } from "node:timers/promises";
import { parallel, percentile, normalizeRoute } from "./metrics.mjs";

const { chromium, request } = createRequire("/app/package.json")("@playwright/test");
const origin = "https://horizon.localhost";
const mailpit = "http://mailpit:8025";
const samples = integer("PERF_SAMPLES", 100, 5, 1000);
const mixedSeconds = integer("PERF_MIXED_SECONDS", 65, 0, 300);
const concurrency = 4;
const today = new Date().toISOString().slice(0, 10);
const day = (offset) =>
  new Date(Date.parse(today) + offset * 86400_000).toISOString().slice(0, 10);
const fixtureId = (key) =>
  createHash("md5")
    .update(`horizon-perf:${key}`)
    .digest("hex")
    .replace(/^(.{8})(.{4})(.{4})(.{4})(.*)$/, "$1-$2-$3-$4-$5");
const paths = {
  calendar7: `/events?start_date=${today}&end_date=${day(7)}`,
  calendar90: `/events?start_date=${today}&end_date=${day(90)}`,
  calendar90Deep: `/events?start_date=${today}&end_date=${day(90)}&offset=150`,
  tasksFirst: "/tasks",
  notificationsFirst: "/notifications",
};

function integer(name, fallback, min, max) {
  const value = Number(process.env[name] ?? fallback);
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new Error(`${name} must be an integer in ${min}–${max}`);
  }
  return value;
}

async function until(check, description) {
  for (let i = 0; i < 100; i++) {
    const value = await check();
    if (value) return value;
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${description}`);
}

async function magicLink(api, email) {
  if ((await api.post("/auth/magic-links", { data: { email, time_zone: "UTC" } })).status() !== 202) {
    throw new Error("Magic link request failed");
  }
  const id = await until(async () => {
    const response = await api.get(`${mailpit}/api/v1/search`, {
      params: { query: `to:${email}` },
    });
    if (!response.ok()) return null;
    const inbox = await response.json();
    return inbox.messages?.find((m) => m.Subject === "Your Horizon sign-in link")?.ID;
  }, "Mailpit");
  const response = await api.get(`${mailpit}/api/v1/message/${encodeURIComponent(id)}`);
  if (!response.ok()) throw new Error("Mailpit message fetch failed");
  const { Text } = await response.json();
  const link = Text.match(/https:\/\/horizon\.localhost\/auth\/complete[^\s<>]+/)?.[0];
  if (!link) throw new Error("Missing local auth callback");
  return link;
}

async function browserStartup(browser, api, profile) {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const page = await context.newPage();
    const link = await magicLink(api, `perf-${profile}@example.test`);
    const redeemed = page.waitForResponse(
      (r) => new URL(r.url()).pathname === "/api/auth/magic-links/redeem",
    );
    try {
      await page.goto(link);
    } catch {
      throw new Error("Browser could not open local auth callback");
    }
    const response = await redeemed;
    if (!response.ok()) throw new Error("Browser sign-in failed");
    const { tokens } = await response.json();
    await page.getByRole("button", { name: "Sign out" }).waitFor();
    await page.waitForLoadState("networkidle");

    const runs = [];
    for (let i = 0; i < 3; i++) {
      const counts = {};
      const statuses = {};
      const count = (event) => {
        const url = new URL(event.url());
        if (url.origin !== origin || !url.pathname.startsWith("/api/")) return;
        const key = `${event.request().method()} ${normalizeRoute(url.pathname)}`;
        counts[key] = (counts[key] ?? 0) + 1;
        statuses[event.status()] = (statuses[event.status()] ?? 0) + 1;
      };
      page.on("response", count);
      const started = performance.now();
      try {
        await page.reload({ waitUntil: "networkidle" });
        await page.getByRole("navigation", { name: "Spaces", exact: true })
          .getByRole("button", { name: new RegExp(`^Perf ${profile} 1 owner$`, "i") })
          .waitFor();
      } finally {
        page.off("response", count);
      }
      if (Object.keys(statuses).some((status) => !status.startsWith("2"))) {
        throw new Error(`Unexpected startup HTTP status for ${profile}`);
      }
      runs.push({
        count: Object.values(counts).reduce((a, b) => a + b, 0),
        requests: counts,
        statuses,
        toNetworkIdleMs: Math.round(performance.now() - started),
      });
    }
    return { runs, accessToken: tokens.access_token };
  } finally {
    await context.close();
  }
}

async function measure(api, path, token, data) {
  const started = performance.now();
  const response = await api.fetch(path, {
    method: data ? "POST" : "GET",
    headers: { Authorization: `Bearer ${token}` },
    data,
  });
  try {
    if (response.status() !== (data ? 201 : 200)) {
      throw new Error(`${normalizeRoute(path)} returned ${response.status()}`);
    }
    const body = await response.body();
    return { ms: performance.now() - started, bytes: body.length };
  } finally {
    await response.dispose();
  }
}

function summarize(observations) {
  const stats = (key) => observations.map((entry) => entry[key]);
  const metric = (key, p) => Number(percentile(stats(key), p).toFixed(2));
  return {
    requests: observations.length,
    p50Ms: metric("ms", 0.5),
    p95Ms: metric("ms", 0.95),
    p99Ms: metric("ms", 0.99),
    medianResponseBytes: metric("bytes", 0.5),
  };
}

async function profileApi(api, name, token) {
  const spaceId = fixtureId(`${name}:space:1`);
  const large = name === "large";
  const inboxOffset = large ? 10000 : 300;
  const boundary = await api.get(`/notifications?limit=1&offset=${inboxOffset - 1}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  let cursor;
  try {
    if (!boundary.ok()) throw new Error("Notification boundary lookup failed");
    cursor = (await boundary.json()).next_cursor;
    if (!cursor) throw new Error("Missing Notification cursor for deep page");
  } finally {
    await boundary.dispose();
  }
  const workloads = {
    ...paths,
    tasksDeep: `/tasks?lists_offset=${large ? 125 : 8}&tasks_offset=${large ? 9000 : 280}`,
    placesFirst: `/places?space_id=${spaceId}`,
    placesDeep: `/places?space_id=${spaceId}&limit=10&offset=${large ? 200 : 50}`,
    notificationsDeep: `/notifications?offset=${inboxOffset}`,
    notificationsDeepCursor: `/notifications?cursor=${encodeURIComponent(cursor)}`,
    eventFanout: `/spaces/${spaceId}/events`,
  };
  const results = {};
  for (const [label, path] of Object.entries(workloads)) {
    let eventNo = 0;
    const run = () =>
      measure(api, path, token, label === "eventFanout" ? {
        title: `Fan-out probe ${eventNo++}`,
        starts_at: `${day(1)}T12:00:00Z`,
        time_zone: "UTC",
        all_day: false,
      } : undefined);
    await parallel(3, 1, run);
    const count = label === "eventFanout" ? Math.min(samples, 12) : samples;
    results[label] = {
      ...summarize(await parallel(count, concurrency, run)),
      warmups: 3,
      concurrency,
    };
    console.log(`${name}/${label}: p95 ${results[label].p95Ms} ms`);
  }
  return results;
}

async function profileMixed(api, token) {
  if (!mixedSeconds) return null;
  const workload = { ...paths, places: `/places?space_id=${fixtureId("large:space:1")}` };
  const observations = Object.fromEntries(Object.keys(workload).map((key) => [key, []]));
  const deadline = performance.now() + mixedSeconds * 1000;
  const start = new Date().toISOString();
  while (performance.now() < deadline) {
    const started = performance.now();
    await Promise.all(Object.entries(workload).map(async ([key, path]) => {
      observations[key].push(await measure(api, path, token));
    }));
    await delay(Math.max(0, 250 - (performance.now() - started)));
  }
  return {
    window: { start, end: new Date().toISOString(), targetSeconds: mixedSeconds },
    workloads: Object.fromEntries(Object.entries(observations).map(([key, values]) => [key, summarize(values)])),
  };
}

async function explainSql() {
  const cases = [
    ["events/repositories.rs", "SELECT e.id", "AT TIME ZONE e.time_zone", "calendar_series", "uuid,date,date",
      `'${fixtureId("large")}', :'date', (:'date'::date + 90)`],
    ["events/repositories.rs", "SELECT o.event_id", null, "calendar_overrides", "uuid,date,date,timestamptz,timestamptz",
      `'${fixtureId("large")}', :'date', (:'date'::date + 90), :'date', (:'date'::date + 91)`],
    ["todos/repositories.rs", "SELECT t.id, l.space_id", "OFFSET $3", "tasks", "uuid,bigint,bigint",
      `'${fixtureId("large")}', 51, 9000`],
    ["places/repositories.rs", "SELECT p.id", "OFFSET $3", "places", "uuid,bigint,bigint,uuid,text",
      `'${fixtureId("large")}', 11, 200, '${fixtureId("large:space:1")}', NULL`],
    ["places/repositories.rs", "SELECT p.category, COUNT(*)", null, "place_category_counts", "uuid,uuid",
      `'${fixtureId("large:space:1")}', '${fixtureId("large")}'`],
    ["notifications/repositories.rs", "SELECT n.id", "OFFSET $3", "notifications", "uuid,bigint,bigint",
      `'${fixtureId("large")}', 51, 10000`],
    ["notifications/repositories.rs", "SELECT n.id", "(n.created_at, n.id) <", "notifications_cursor", "uuid,timestamptz,uuid,bigint",
      `'${fixtureId("large")}', :'notification_boundary_time', :'notification_boundary_id', 51`],
    ["notifications.rs", "INSERT INTO notifications", "SELECT $1, $2, $3", "notification_insert", "uuid,uuid,uuid,text,text,text,text",
      `gen_random_uuid(), '${fixtureId("large:peer:1")}', '${fixtureId("large:space:1")}', 'event_change', 'Plan probe', 'Plan probe', 'calendar'`],
    ["task_reminders.rs", "WITH batch AS MATERIALIZED", "e.recipient_id, e.offsets_minutes", "reminder_candidates", "uuid,boolean,bigint",
      "'00000000-0000-0000-0000-000000000000', true, 256"],
    ["task_reminders.rs", "WITH batch AS MATERIALIZED", "UPDATE task_reminder_jobs", "reminder_cancel", "uuid,boolean,bigint",
      "'00000000-0000-0000-0000-000000000000', true, 256"],
    ["task_reminders.rs", "UPDATE task_reminder_jobs SET status = 'sending'", null, "reminder_claim", "timestamptz", "now()"],
  ];
  const statements = [
    "SET TIME ZONE 'UTC';",
    `SELECT created_at AS notification_boundary_time, id AS notification_boundary_id FROM notifications WHERE recipient_id = '${fixtureId("large")}' ORDER BY created_at DESC, id DESC OFFSET 9999 LIMIT 1 \\gset`,
  ];
  for (const [file, prefix, contains, name, types, args] of cases) {
    const source = await readFile(`/api-source/${file}`, "utf8");
    // Pull the exact SQL string from the implementation; a changed seam fails
    // visibly rather than silently benchmarking an outdated copy of a query.
    const matches = [...source.matchAll(/"((?:[^"\\]|\\[\s\S])*)"/g)]
      .map((match) => match[1].replace(/\\\r?\n[ \t]*/g, ""))
      .filter((sql) => sql.startsWith(prefix) && (!contains || sql.includes(contains)));
    if (matches.length !== 1) throw new Error(`Could not identify SQL for ${name}`);
    statements.push(
      `\\echo ==== ${name} ====`,
      "BEGIN;",
      ...(types ? [`PREPARE ${name} (${types}) AS ${matches[0]};`] : []),
      `EXPLAIN (ANALYZE, BUFFERS, SETTINGS, TIMING OFF) ${types ? `EXECUTE ${name} (${args})` : matches[0]};`,
      ...(types ? [`DEALLOCATE ${name};`] : []),
      "ROLLBACK;",
    );
  }
  return statements.join("\n") + "\n";
}

async function main() {
  await writeFile("/results/explain.sql", await explainSql());
  await writeFile("/results/day", today);
  const api = await request.newContext({ baseURL: "http://api:8000", timeout: 10_000 });
  let browser;
  try {
    await until(async () => {
      try { return (await api.get("/health/ready", { timeout: 1000 })).ok(); } catch { return false; }
    }, "API readiness");
    browser = await chromium.launch();
    const profiles = {};
    let largeToken;
    for (const name of ["typical", "large"]) {
      const { accessToken, runs } = await browserStartup(browser, api, name);
      profiles[name] = { browserStartup: runs, api: await profileApi(api, name, accessToken) };
      if (name === "large") largeToken = accessToken;
    }
    if (mixedSeconds) console.log(`Measuring mixed reads for ${mixedSeconds} seconds`);
    const mixed = await profileMixed(api, largeToken);
    await writeFile("/results/mixed-start", mixed?.window.start ?? "");
    await writeFile("/results/mixed-end", mixed?.window.end ?? "");
    await writeFile("/results/baseline.json", JSON.stringify({
      revision: process.env.PERF_REVISION,
      architecture: process.arch,
      dockerCpus: availableParallelism(),
      dockerMemoryBytes: totalmem(),
      sampleDateUtc: today,
      profiles,
      mixed,
    }, null, 2) + "\n");
  } finally {
    await browser?.close();
    await api.dispose();
  }
}

await main();
