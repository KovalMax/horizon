import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e-smoke",
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  reporter: "list",
  use: {
    ...devices["Desktop Chrome"],
    baseURL: process.env.HORIZON_SMOKE_URL ?? "https://horizon.localhost",
    ignoreHTTPSErrors: true,
    trace: "retain-on-failure",
  },
});
