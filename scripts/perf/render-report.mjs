import { readFile, writeFile } from "node:fs/promises";

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
const reconcile = db.statements.find((r) => r.query.startsWith("SELECT t.id AS task_id"));
lines.push(
  "",
  `- Per-recipient Notification insert calls: **${fanout?.calls ?? 0}** across both fan-out workloads (including warmups).`,
  `- Reminder candidate scans: **${reconcile?.calls ?? 0}** across the full run; **${db.reminder_jobs}** jobs present at the end (${db.reminder_jobs_created_during_mixed} newly created in the mixed window).`,
);
lines.push("", "See `query-plans.txt` for executed plans with buffers, and `fixtures.txt` for row counts.", "");
await writeFile("/results/summary.md", lines.join("\n"));
