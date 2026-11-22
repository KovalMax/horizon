export function percentile(values, fraction) {
  if (!values.length) throw new Error("Cannot summarize an empty sample");
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.ceil(sorted.length * fraction) - 1];
}

export function normalizeRoute(path) {
  return path.replace(/\b[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\b/gi, ":id");
}

export async function parallel(count, concurrency, task) {
  const results = [];
  let index = 0;
  await Promise.all(Array.from({ length: Math.min(count, concurrency) }, async () => {
    while (index < count) {
      const slot = index++;
      results[slot] = await task();
    }
  }));
  return results;
}
