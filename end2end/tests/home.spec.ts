import { test, expect } from "@playwright/test";
import { goto } from "./helpers";

test("homepage has the app title", async ({ page }) => {
  await goto(page, "/");

  await expect(page).toHaveTitle("Guidance");
});

test("sidebar navigates between sections", async ({ page }) => {
  await goto(page, "/");

  // Exact match: the homepage's own quick-link cards also expose
  // "Compendium"/"Characters" as part of their accessible name (with a
  // leading index code), so a substring match would be ambiguous here.
  await page.getByRole("link", { name: "Compendium", exact: true }).click();
  await expect(page).toHaveURL("/compendium");

  await page.getByRole("link", { name: "Characters", exact: true }).click();
  await expect(page).toHaveURL("/characters");
});

test("unknown routes fall back to not found", async ({ page }) => {
  await goto(page, "/no-such-page");

  await expect(page.getByText("Page not found.")).toBeVisible();
});
