import { test, expect } from "@playwright/test";
import { goto } from "./helpers";

// Overrides the project's shared authenticated storageState (set up by
// auth.setup.ts) so these tests exercise the actual unauthenticated flow.
test.use({ storageState: { cookies: [], origins: [] } });

test("login page has no sign-up link", async ({ page }) => {
  await goto(page, "/login");

  await expect(page.getByRole("heading", { name: "Sign in to Guidance" })).toBeVisible();
  await expect(page.getByRole("link", { name: /sign up|create account/i })).toHaveCount(0);
});

test("visiting a protected route while logged out redirects to login", async ({ page }) => {
  await goto(page, "/compendium");

  await expect(page).toHaveURL("/login");
});

test("wrong credentials show an error and stay on the login page", async ({ page }) => {
  await goto(page, "/login");

  await page.getByLabel("Username").pressSequentially("admin");
  await page.getByLabel("Password").pressSequentially("not-the-password");
  await page.getByRole("button", { name: "Log in" }).click();

  await expect(page.getByText("Invalid username or password")).toBeVisible();
  await expect(page).toHaveURL("/login");
});

test("correct credentials log the admin in", async ({ page }) => {
  await goto(page, "/login");

  await page.getByLabel("Username").pressSequentially("admin");
  await page.getByLabel("Password").pressSequentially("admin");
  await page.getByRole("button", { name: "Log in" }).click();

  await expect(page).toHaveURL("/");
  await expect(page.getByText("Logged in as admin")).toBeVisible();
});
