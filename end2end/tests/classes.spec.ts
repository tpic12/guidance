import { test, expect } from "@playwright/test";
import { goto } from "./helpers";

test("index lists the seeded classes", async ({ page }) => {
  await goto(page, "/compendium/classes");

  for (const name of ["Fake Mage", "Fake Warrior"]) {
    await expect(page.getByRole("link", { name: new RegExp(name) })).toBeVisible();
  }
});

test("class cards summarize hit die and saving throws", async ({ page }) => {
  await goto(page, "/compendium/classes");

  const warrior = page.getByRole("link", { name: /Fake Warrior/ });
  await expect(warrior).toContainText("Hit Die: d10");
  await expect(warrior).toContainText("Saving Throws: Strength, Constitution");
  await expect(warrior).toContainText("Subclasses: 1");
});

test("clicking a class opens its detail page", async ({ page }) => {
  await goto(page, "/compendium/classes");

  await page.getByRole("link", { name: /Fake Warrior/ }).click();
  await expect(page).toHaveURL("/compendium/classes/fake-warrior");
  await expect(page.locator("h1")).toContainText("Fake Warrior");
});
