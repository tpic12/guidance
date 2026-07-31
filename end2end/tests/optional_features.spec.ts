import { test, expect } from "@playwright/test";
import { goto, applyFilters, columnHeader, detailCard, search, tableRows } from "./helpers";

test.beforeEach(async ({ page }) => {
  await goto(page, "/compendium/optional-features");
  await tableRows(page).first().waitFor();
});

test("table lists the seeded optional features with prerequisites", async ({ page }) => {
  await expect(tableRows(page)).toHaveCount(10);

  const surge = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Arcane Surge", exact: true }),
  });
  await expect(surge.locator("td:nth-child(3)")).toHaveText("Fake Mage level 5");

  const burst = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Elemental Burst", exact: true }),
  });
  await expect(burst.locator("td:nth-child(3)")).toHaveText("Fake Warrior (Fake Fist) level 3");

  const sight = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Sight", exact: true }),
  });
  await expect(sight.locator("td:nth-child(3)")).toHaveText("—");
});

test("search narrows the table by name", async ({ page }) => {
  await search(page, "Search optional features...", "elemental");

  await expect(tableRows(page)).toHaveCount(1);
});

test("clicking a row opens the optional feature's detail card", async ({ page }) => {
  await page.getByRole("cell", { name: "Fake Deep Cut", exact: true }).click();

  const card = detailCard(page);
  await expect(card.locator(".card-title")).toContainText("Fake Deep Cut");
  await expect(card).toContainText("Costs: Fake Superiority Die");
  await expect(card).toContainText(
    "you can expend one Fake Superiority Die to add it to the damage roll",
  );
});

test("prerequisite variants render correctly: pact and required spell", async ({ page }) => {
  const bond = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Blade Bond", exact: true }),
  });
  await expect(bond.locator("td:nth-child(3)")).toHaveText("Pact of the Fake Chain");

  const empowered = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Empowered Bolt", exact: true }),
  });
  await expect(empowered.locator("td:nth-child(3)")).toHaveText("fake bolt known");
});

test("type filter limits rows to the chosen feature type", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Maneuver", exact: true }).click();
  });

  await expect(tableRows(page)).toHaveCount(2);
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Deep Cut");
  await expect(tableRows(page).nth(1).locator("td:first-child")).toHaveText("Fake Trip Attack");
});

test("the Fighting Style type filter matches every class variant at once", async ({ page }) => {
  // Fake Weapon Focus is tagged Fighter+Ranger, Fake Guard Stance is
  // Fighter-only — one grouped "Fighting Style" chip should catch both,
  // proving the filter expands to every underlying class-scoped type.
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Fighting Style", exact: true }).click();
  });

  await expect(tableRows(page)).toHaveCount(2);
  await expect(tableRows(page).filter({ hasText: "Fake Weapon Focus" })).toHaveCount(1);
  await expect(tableRows(page).filter({ hasText: "Fake Guard Stance" })).toHaveCount(1);
});

test("required class/subclass filter limits rows to the chosen requirement", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Fake Warrior (Fake Fist)", exact: true }).click();
  });

  await expect(tableRows(page)).toHaveCount(1);
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Elemental Burst");
});

test("required pact filter limits rows to the chosen pact", async ({ page }) => {
  await applyFilters(page, async (dialog) => {
    await dialog.getByRole("button", { name: "Fake Chain", exact: true }).click();
  });

  await expect(tableRows(page)).toHaveCount(1);
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Blade Bond");
});

test("a feature spanning multiple Fighting Style variants shows one deduped badge", async ({ page }) => {
  const focus = tableRows(page).filter({
    has: page.getByRole("cell", { name: "Fake Weapon Focus", exact: true }),
  });
  const badges = focus.locator("td:nth-child(2) .badge");
  await expect(badges).toHaveCount(1);
  await expect(badges).toHaveText("Fighting Style");
});

test("name column sorts descending on second click", async ({ page }) => {
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Fake Arcane Surge");

  await columnHeader(page, "Name").click(); // already NameAsc -> NameDesc
  await expect(tableRows(page).first().locator("td:first-child")).toHaveText("Pact of the Fake Chain");
});
