import { test, expect } from "@playwright/test";
import { goto, applyFilters, columnHeader, detailCard, search, tableRows } from "./helpers";

test.beforeEach(async ({ page }) => {
  await goto(page, "/compendium/species");
  await tableRows(page).first().waitFor();
});

test("table lists the seeded species, skipping the _copy stub", async ({ page }) => {
  // 5 rows: Skyfolk, Tunnelkin, Mimicborn, and both Duskkin reprints (TBK +
  // ZBK) — the "(Renovated)" _copy stub is excluded, not a 6th row.
  await expect(tableRows(page)).toHaveCount(5);

  const skyfolk = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Skyfolk", exact: true }),
  });
  await expect(skyfolk.locator("td:nth-child(2)")).toHaveText("Dexterity +2, Wisdom +1");
  await expect(skyfolk.locator("td:nth-child(3)")).toHaveText("Medium");

  const tunnelkin = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Tunnelkin", exact: true }),
  });
  await expect(tunnelkin.locator("td:nth-child(2)")).toHaveText("Constitution +2, Choose any +1");
  await expect(tunnelkin.locator("td:nth-child(3)")).toHaveText("Small");
});

test("search narrows the table by name", async ({ page }) => {
  await search(page, "Search species...", "sky");

  await expect(tableRows(page)).toHaveCount(1);
});

test("clicking a row opens the species' detail card", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Mimicborn", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Mimicborn");
  await expect(card).toContainText(
    "Ability Scores: Charisma +2, Choose Strength or Dexterity +1",
  );
  await expect(card).toContainText("Size: Small or Medium");
  await expect(card).toContainText("Speed: 30 ft., swim 20 ft.");
  await expect(card.locator("table caption")).toHaveText("Convincing Furniture");
});

test("source filter limits rows to the chosen book", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "TBK", exact: true }).click();
  });

  // Skyfolk, Tunnelkin, and the TBK printing of Duskkin.
  await expect(tableRows(page)).toHaveCount(3);
  for (const source of await tableRows(page).locator("td:nth-child(4)").allTextContents()) {
    expect(source).toBe("TBK");
  }
});

test("name column sorts descending on second click", async ({ page }) => {
  // Alphabetically first is now Duskkin (D < M), added to exercise the
  // character builder's species-grouping UI with a genuine reprint pair.
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Duskkin");

  await columnHeader(page, "Name").click(); // already NameAsc -> NameDesc
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Tunnelkin");
});
