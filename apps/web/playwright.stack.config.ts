import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e-stack",
  forbidOnly: true,
  workers: 1,
  timeout: 90_000,
  reporter: "list",
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "https://horizon.localhost",
    ignoreHTTPSErrors: true, // The isolated stack generates its own local CA.
    trace: "off", // Auth callbacks and join links must not reach CI artifacts.
    video: "off",
    screenshot: "off",
  },
});
