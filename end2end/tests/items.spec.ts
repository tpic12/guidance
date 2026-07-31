import { test, expect } from "@playwright/test";
import { goto, applyFilters, columnHeader, detailCard, search, tableRows } from "./helpers";

test.beforeEach(async ({ page }) => {
  await goto(page, "/compendium/items");
  await tableRows(page).first().waitFor();
});

test("table lists the seeded items", async ({ page }) => {
  await expect(tableRows(page)).toHaveCount(4);

  const club = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Club", exact: true }),
  });
  await expect(club.locator("td:nth-child(3)")).toHaveText("None");
  await expect(club.locator("td:nth-child(4)")).toHaveText("1 sp");

  const dagger = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Sizzling Dagger", exact: true }),
  });
  await expect(dagger.locator("td:nth-child(3)")).toHaveText("Rare");
  await expect(dagger.locator("td:nth-child(4)")).toHaveText("1 gp");
});

test("search narrows the table by name", async ({ page }) => {
  await search(page, "Search items...", "dagger");

  await expect(tableRows(page)).toHaveCount(2);
});

test("clicking a row opens the item's detail card", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Club", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Club");
  await expect(card).toContainText("Melee Weapon");
});

test("a _copy + insertArr derived item inherits parent fields and appends its own entry", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Sizzling Dagger (Awakened)", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Sizzling Dagger (Awakened)");
  // Inherited from the parent via `_copy`:
  await expect(card).toContainText("Rare");
  await expect(card).toContainText("This invented dagger crackles faintly");
  // Own field overriding the parent's, plus the spliced `_mod.insertArr` entry:
  await expect(card).toContainText("+2 to attack and damage rolls");
  await expect(card).toContainText("the dagger's bonus increases to +2");
});

test("an item group's detail card lists its concrete members", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Rings of Testing", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Rings of Testing");
  await expect(card.locator(".card-title")).toContainText("Item Group");
  await expect(card).toContainText("Fake Ring of Testing, Alpha");
  await expect(card).toContainText("Fake Ring of Testing, Beta");
});

test("property filter limits rows to items carrying that property", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Finesse", exact: true }).click();
  });

  await expect(tableRows(page)).toHaveCount(2);
});

test("rarity filter limits rows to the chosen rarity", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Rare", exact: true }).click();
  });

  await expect(tableRows(page)).toHaveCount(2);
});

test("name column sorts descending on second click", async ({ page }) => {
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Club");

  await columnHeader(page, "Name").click(); // already NameAsc -> NameDesc
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Sizzling Dagger (Awakened)");
});
