import { readFile, writeFile } from "node:fs/promises";
import { percentile } from "./metrics.mjs";

const read = (name) => readFile(`/results/${name}`, "utf8");
const http = JSON.parse(await read("baseline.json"));
const db = JSON.parse(await read("postgres.json"));
const lines = [
  "# Horizon performance baseline",
  "",
  `Revision: \`${http.revision}\` · ${http.architecture} · ${http.dockerCpus} Docker CPUs · ${(http.dockerMemoryBytes / 2 ** 30).toFixed(1)} GiB · PostgreSQL ${db.postgres_version} · UTC ${http.sampleDateUtc}`,
  "",
  "Report only. These are observations from this run, not production SLOs.",
  "",
  "## Signed-in All Spaces reload",
];
for (const [name, profile] of Object.entries(http.profiles)) {
  lines.push("", `**${name}:** ${profile.browserStartup.map((r) => r.count).join(", ")} API responses over three reloads.`);
  for (const [route, count] of Object.entries(profile.browserStartup[0].requests)
    .filter(([, count]) => count > 1)) {
    lines.push(`- First reload repeated \`${route}\` ${count} times.`);
  }
}
lines.push("", "## Endpoint timings ranked by p95", "",
  "| Account | Workload | Requests | p50 ms | p95 ms | p99 ms | Median body bytes |",
  "| --- | --- | ---: | ---: | ---: | ---: | ---: |");
const workloads = Object.entries(http.profiles).flatMap(([profile, { api }]) =>
  Object.entries(api).map(([name, stats]) => ({ profile, name, ...stats })),
);
for (const item of workloads.sort((a, b) => b.p95Ms - a.p95Ms)) {
  lines.push(`| ${item.profile} | ${item.name} | ${item.requests} | ${item.p50Ms} | ${item.p95Ms} | ${item.p99Ms} | ${item.medianResponseBytes} |`);
}
if (http.mixed) {
  lines.push("", "## Mixed reads while the reminder worker runs", "",
    `Window: ${http.mixed.window.start} – ${http.mixed.window.end}.`, "",
    "| Workload | Requests | p95 ms |",
    "| --- | ---: | ---: |");
  for (const [name, stats] of Object.entries(http.mixed.workloads)) {
    lines.push(`| ${name} | ${stats.requests} | ${stats.p95Ms} |`);
  }
}
lines.push("", "## PostgreSQL", "",
  "Statements ranked by total execution time, including startup and the worker. `postgres.json` contains complete query texts and buffer counters.", "",
  "| Statement (abbreviated) | Calls | Total ms | Mean ms | Shared reads | Temp writes |",
  "| --- | ---: | ---: | ---: | ---: | ---: |");
for (const row of db.statements.slice(0, 10)) {
  const label = row.query.replace(/\s+/g, " ").replace(/[`|]/g, " ").slice(0, 100);
  lines.push(`| \`${label}\` | ${row.calls} | ${row.total_ms} | ${row.mean_ms} | ${row.shared_blks_read} | ${row.temp_blks_written} |`);
}
const fanout = db.statements.find((r) => r.query.startsWith("INSERT INTO notifications") && r.query.includes("notification_preferences"));
const reconcile = db.statements.find((r) => r.query.startsWith("WITH batch AS MATERIALIZED") && r.query.includes("e.recipient_id, e.offsets_minutes"));
const cancel = db.statements.find((r) => r.query.startsWith("WITH batch AS MATERIALIZED") && r.query.includes("UPDATE task_reminder_jobs"));
const upsert = db.statements.find((r) => r.query.startsWith("INSERT INTO task_reminder_jobs"));
lines.push(
  "",
  `- Per-recipient Notification insert calls: **${fanout?.calls ?? 0}** across both fan-out workloads (including warmups).`,
  `- Task reminder reconciliation: **${reconcile?.calls ?? 0}** candidate reads, **${cancel?.calls ?? 0}** cancellation checks, **${upsert?.calls ?? 0}** upserts affecting **${upsert?.rows ?? 0}** rows.`,
  `- Task reminder jobs present at the end: **${db.reminder_jobs}** (${db.reminder_jobs_created_during_mixed} newly created in the mixed window).`,
);
lines.push("", "See `query-plans.txt` for executed plans with buffers, and `fixtures.txt` for row counts.", "");
const profileLines = (await read("calendar-profile.log")).trim().split("\n");
const phases = ["series_us", "overrides_us", "build_us", "moved_overrides_us", "sort_us", "page_us"];
const groups = new Map();
for (const line of profileLines) {
  const sample = Object.fromEntries(
    [...line.matchAll(/\b(\w+)=(\d+)/g)].map(([, key, value]) => [key, Number(value)]),
  );
  if (!sample.days || !phases.every((phase) => Number.isFinite(sample[phase]))) {
    throw new Error("Missing or incomplete Calendar phase profile");
  }
  const cohort = sample.series >= 1000 ? "large" : "typical";
  const key = `${cohort}, ${sample.days} days`;
  if (!groups.has(key)) groups.set(key, []);
  groups.get(key).push(sample);
}
lines.push("## Calendar range phases (p50, ms)", "",
  "The API measures time spent awaiting its two queries (including pool wait/decoding), generating occurrences, processing moved overrides, sorting, and preparing the page. This excludes HTTP serialization and network time. Rows include warmups and mixed reads.", "",
  "| Cohort and window | Requests | Median occurrences | Series read | Overrides read | Build | Moved overrides | Sort | Prepare page |",
  "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
for (const [key, rows] of [...groups].sort(([a], [b]) => a.localeCompare(b))) {
  const median = (field) => percentile(rows.map((row) => row[field]), 0.5);
  lines.push(`| ${key} | ${rows.length} | ${median("occurrences")} | ${phases.map((field) => (median(field) / 1000).toFixed(2)).join(" | ")} |`);
}
lines.push("", "Candidate cohorts are grouped by whether the query returns at least 1,000 series/single-event rows.", "");

const backlogStart = await read("reminder-start.csv").catch((error) => {
  if (error.code === "ENOENT") return null;
  throw error;
});
if (backlogStart) {
  const parseSample = (line) => line.trim().split(",").map(Number);
  const start = parseSample(backlogStart);
  const end = parseSample(await read("reminder-end.csv"));
  const samples = (await read("reminder-samples.txt")).split("\n")
    .filter((line) => /^\d{13}(,\d+){8}$/.test(line)).map(parseSample);
  const probes = [...(await read("reminder-pool.log")).matchAll(
    /epoch_ms=(\d+) acquire_us=(\d+) acquired=([01]) peak_rss_kib=(\d+)/g,
  )].map((match) => match.slice(1).map(Number));
  const lockLogs = (await read("reminder-lock-log-counts.txt")).trim();
  if (!samples.length || !probes.length) throw new Error("Incomplete reminder backlog telemetry");
  const windowStart = http.mixed ? Date.parse(http.mixed.window.start) : start[0];
  const windowEnd = http.mixed ? Date.parse(http.mixed.window.end) : end[0];
  const inWindow = (sample) => sample[0] >= windowStart && sample[0] <= windowEnd;
  const poolInWindow = probes.filter(inWindow);
  const acquireMs = poolInWindow.filter((probe) => probe[2] === 1).map((probe) => probe[1] / 1000);
  const firstCalendar = samples.find((sample) => sample[7] > start[7]);
  lines.push("## Reminder backlog", "",
    `- Full observation: ${((end[0] - start[0]) / 1000).toFixed(1)} s, ${samples.length} PostgreSQL samples.`,
    `- Task jobs: ${start[1]} pending at start; ${end[3]} sent, ${end[1]} pending, ${end[2]} sending, ${end[4]} failed/cancelled and ${end[5]} retried at end.`,
    `- Oldest pending age: ${(start[6] / 1000).toFixed(1)} s → ${(end[6] / 1000).toFixed(1)} s (already aged at seeding).`,
    `- Calendar: ${end[7]} deliveries by the end${firstCalendar ? `; first observed after ${((firstCalendar[0] - start[0]) / 1000).toFixed(2)} s, with ${firstCalendar[1] + firstCalendar[2]} Task jobs outstanding` : ""}.`,
    `- Peak API RSS: ${(Math.max(...probes.map((probe) => probe[3])) / 1024).toFixed(1)} MiB (${probes.length} samples).`,
    `- Pool checkout probe during the measured window: ${poolInWindow.length} samples, ${poolInWindow.length - acquireMs.length} failed${acquireMs.length ? `; p50 ${percentile(acquireMs, 0.5).toFixed(3)} ms, p95 ${percentile(acquireMs, 0.95).toFixed(3)} ms, p99 ${percentile(acquireMs, 0.99).toFixed(3)} ms` : ""}.`,
    `- Database lock waits: ${samples.filter((sample) => sample[8] > 0).length}/${samples.length} samples had a waiting session; ${lockLogs ? "see reminder-lock-log-counts.txt for waits logged at 50 ms or more" : "none logged at the 50 ms threshold"}.`,
    "",
    "This is a local synthetic run with Mailpit, no production latency claim and no worker-off control. Pool checkout is a probe, not an attribution of real request wait time. Short lock waits can go unobserved.", "",
  );
}

await writeFile("/results/summary.md", lines.join("\n"));
