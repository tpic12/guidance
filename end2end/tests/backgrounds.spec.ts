import { test, expect } from "@playwright/test";
import { goto, applyFilters, columnHeader, detailCard, search, tableRows } from "./helpers";

test.beforeEach(async ({ page }) => {
  await goto(page, "/compendium/backgrounds");
  await tableRows(page).first().waitFor();
});

test("table lists the seeded backgrounds with their skills", async ({ page }) => {
  await expect(tableRows(page)).toHaveCount(3);

  const scholar = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Scholar", exact: true }),
  });
  await expect(scholar.locator("td:nth-child(2)")).toHaveText("History, Insight");
});

test("search narrows the table by name", async ({ page }) => {
  await search(page, "Search backgrounds...", "scholar");

  await expect(tableRows(page)).toHaveCount(2); // Fake Scholar, Fake City Scholar
});

test("clicking a row opens the background's detail card", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Scholar", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Scholar");
  await expect(card).toContainText("Skill Proficiencies:");
  await expect(card).toContainText("Feature: Fake Erudition");
});

test("variant backgrounds resolve their copied entries", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake City Scholar", exact: true }).click();

  const card = detailCard(page);
  await expect(card).toContainText("Skill Proficiencies:");
  await expect(card).toContainText("Feature: Fake City Erudition");
});

test("source filter limits rows to the chosen book", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "TBK", exact: true }).click();
  });

  await expect(tableRows(page)).toHaveCount(2);
  for (const source of await tableRows(page).locator("td:nth-child(3)").allTextContents()) {
    expect(source).toBe("TBK");
  }
});

test("source column sorts ascending then descending", async ({ page }) => {
  await columnHeader(page, "Source").click();
  await expect(tableRows(page).first().locator("td:nth-child(3)")).toHaveText("TBK");

  await columnHeader(page, "Source").click();
  await expect(tableRows(page).first().locator("td:nth-child(3)")).toHaveText("ZBK");
});
