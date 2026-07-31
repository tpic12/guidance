import { test, expect } from "@playwright/test";
import { goto } from "./helpers";

const sections = [
  { label: "Classes", url: "/compendium/classes" },
  { label: "Spells", url: "/compendium/spells" },
  { label: "Items", url: "/compendium/items" },
  { label: "Backgrounds", url: "/compendium/backgrounds" },
  { label: "Feats", url: "/compendium/feats" },
  { label: "Species", url: "/compendium/species" },
  { label: "Optional Features", url: "/compendium/optional-features" },
];

test("landing page shows a card per section", async ({ page }) => {
  await goto(page, "/compendium");

  for (const section of sections) {
    await expect(page.getByRole("link", { name: section.label })).toBeVisible();
  }
});

for (const section of sections) {
  test(`${section.label} card navigates to its section`, async ({ page }) => {
    await goto(page, "/compendium");

    await page.getByRole("link", { name: section.label }).click();
    await expect(page).toHaveURL(section.url);
  });
}

test("sections link back to the compendium landing page", async ({ page }) => {
  await goto(page, "/compendium/spells");

  await page.getByRole("link", { name: "← Back to Compendium" }).click();
  await expect(page).toHaveURL("/compendium");
});
