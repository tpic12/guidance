import { test, expect } from "@playwright/test";
import { goto, applyFilters, columnHeader, detailCard, search, tableRows } from "./helpers";

test.beforeEach(async ({ page }) => {
  await goto(page, "/compendium/feats");
  await tableRows(page).first().waitFor();
});

test("table lists the seeded feats with prerequisites", async ({ page }) => {
  await expect(tableRows(page)).toHaveCount(3);

  const brawler = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Brawler", exact: true }),
  });
  await expect(brawler.locator("td:nth-child(2)")).toHaveText("Strength 13 or higher");

  const vigilance = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Vigilance", exact: true }),
  });
  await expect(vigilance.locator("td:nth-child(2)")).toHaveText("—");
});

test("search narrows the table by name", async ({ page }) => {
  await search(page, "Search feats...", "brawler");

  await expect(tableRows(page)).toHaveCount(1);
});

test("clicking a row opens the feat's detail card", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Mark", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Mark");
  await expect(card).toContainText("Prerequisite: No other fake mark");
  await expect(card).toContainText(
    "Increase your Constitution score by 1, to a maximum of 20.",
  );
  await expect(card.locator("table caption")).toHaveText("Fake Mark Quirks");
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

test("name column sorts descending on second click", async ({ page }) => {
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Brawler");

  await columnHeader(page, "Name").click(); // already NameAsc -> NameDesc
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Vigilance");
});
