import { devices, defineConfig } from "@playwright/test";

/**
 * The tests assert against the committed mock content in test-fixtures/ (not
 * real rulebook fixtures, which aren't committed). Seed a dedicated database
 * and run the suite against it:
 *
 *   FIXTURES_DIR=test-fixtures DATABASE_URL=sqlite://e2e.db SEED_RESET=1 \
 *     cargo run --features ssr --bin seed
 *   DATABASE_URL=sqlite://e2e.db cargo leptos end-to-end
 */
export default defineConfig({
  testDir: "./tests",
  timeout: 30 * 1000,
  expect: {
    timeout: 5000,
  },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    actionTimeout: 0,
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
  },

  /* Only chromium is installed locally; add firefox/webkit back after
     `npx playwright install firefox webkit`. */
  projects: [
    {
      name: "setup",
      testMatch: /auth\.setup\.ts/,
    },
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        storageState: "./.auth/admin.json",
      },
      dependencies: ["setup"],
    },
  ],
});
