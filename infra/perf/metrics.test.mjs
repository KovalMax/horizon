import assert from "node:assert/strict";
import test from "node:test";
import { parallel, percentile, normalizeRoute } from "./metrics.mjs";

test("percentiles use nearest rank without modifying input", () => {
  const sample = [44, 2, 18, 5];
  assert.equal(percentile(sample, 0.5), 5);
  assert.equal(percentile(sample, 0.95), 44);
  assert.deepEqual(sample, [44, 2, 18, 5]);
  assert.throws(() => percentile([], 0.95), /empty/);
});

test("startup aggregation strips resource identifiers", () => {
  assert.equal(
    normalizeRoute("/api/spaces/aaaaaaaa-1111-2222-3333-bbbbbbbbbbbb/members"),
    "/api/spaces/:id/members",
  );
});

test("a bounded workload performs each requested operation once", async () => {
  let calls = 0;
  let inFlight = 0;
  let max = 0;
  const values = await parallel(7, 3, async () => {
    max = Math.max(max, ++inFlight);
    const sequence = ++calls;
    await Promise.resolve();
    inFlight--;
    return sequence;
  });
  assert.deepEqual(values, [1, 2, 3, 4, 5, 6, 7]);
  assert.equal(max, 3);
});
