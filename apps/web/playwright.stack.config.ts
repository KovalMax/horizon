import { defineConfig, devices } from "@playwright/test";

const tlsSpki = process.env.HORIZON_BROWSER_TLS_SPKI;

export default defineConfig({
  testDir: "./e2e-stack",
  forbidOnly: true,
  workers: 1,
  timeout: 90_000,
  reporter: "list",
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "https://horizon.localhost",
    // The service worker fetch runs outside Playwright's page TLS override.
    launchOptions: tlsSpki
      ? { args: [`--ignore-certificate-errors-spki-list=${tlsSpki}`] }
      : {},
    trace: "off", // Auth callbacks and join links must not reach CI artifacts.
    video: "off",
    screenshot: "off",
  },
});
