import { test, expect } from "@playwright/test";
import { goto, applyFilters, columnHeader, detailCard, search, tableRows } from "./helpers";

test.beforeEach(async ({ page }) => {
  await goto(page, "/compendium/spells");
  await tableRows(page).first().waitFor();
});

test("table lists the seeded spells", async ({ page }) => {
  await expect(tableRows(page)).toHaveCount(7);
});

test("search narrows the table by name", async ({ page }) => {
  await search(page, "Search spells...", "fake bolt");

  await expect(tableRows(page)).toHaveCount(2); // Fake Bolt, Greater Fake Bolt
  await expect(tableRows(page).first()).toContainText("Fake Bolt");
});

test("clicking a row opens the spell's detail card", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Blast", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Blast");
  await expect(card).toContainText("3rd-level evocation");
  await expect(card).toContainText("Casting Time: 1 action");
  await expect(card).toContainText("Range: 150 feet");
  await expect(card).toContainText("Components: V, S, M (a pinch of glitter)");
  await expect(card).toContainText("At Higher Levels");
});

test("school and level filters combine", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Evocation", exact: true }).click();
    await dialog.getByRole("button", { name: "Cantrip", exact: true }).click();
  });

  await expect(page.getByRole("button", { name: "Filters (2)" })).toBeVisible();
  await expect(tableRows(page)).toHaveCount(2); // Fake Bolt, Fake Shimmer
  for (const school of await tableRows(page).locator("td:nth-child(3)").allTextContents()) {
    expect(school).toBe("Evocation");
  }
  for (const level of await tableRows(page).locator("td:nth-child(2)").allTextContents()) {
    expect(level).toBe("Cantrip");
  }
});

test("class filter narrows the table to spells linked to that class", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Fake Paladin", exact: true }).click();
  });

  await expect(page.getByRole("button", { name: "Filters (1)" })).toBeVisible();
  // Fake Paladin links to everything except the two Fake Mage-only cantrips.
  // (`hasText` is a substring match, so "Fake Bolt" would also catch
  // "Greater Fake Bolt" — use an exact cell match instead.)
  await expect(tableRows(page)).toHaveCount(5);
  await expect(tableRows(page).filter({ has: page.getByRole("cell", { name: "Fake Bolt", exact: true }) })).toHaveCount(0);
  await expect(tableRows(page).filter({ has: page.getByRole("cell", { name: "Fake Shimmer", exact: true }) })).toHaveCount(0);
});

test("cancelling the filter dialog keeps the applied filters", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Evocation", exact: true }).click();
  });
  await expect(tableRows(page)).toHaveCount(4); // the evocation spells

  await page.getByRole("button", { name: /^Filters/ }).click();
  const dialog = page.locator("dialog[open]");
  await dialog.getByRole("button", { name: "Reset" }).first().click();
  await dialog.getByRole("button", { name: "Cancel" }).click();

  await expect(page.getByRole("button", { name: "Filters (1)" })).toBeVisible();
  await expect(tableRows(page)).toHaveCount(4);
});

test("level column sorts ascending then descending", async ({ page }) => {
  await columnHeader(page, "Level").click();
  await expect(tableRows(page).first().locator("td:nth-child(2)")).toHaveText("Cantrip");

  await columnHeader(page, "Level").click();
  await expect(tableRows(page).first().locator("td:nth-child(2)")).toHaveText("9");
});

test("a search with no matches says so", async ({ page }) => {
  await search(page, "Search spells...", "zzzzzz");

  await expect(page.getByText("No spells match your filters.")).toBeVisible();
});
