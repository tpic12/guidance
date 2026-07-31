import type { Locator, Page } from "@playwright/test";

/** Waits for hydration: SSR content renders before event handlers attach,
 *  and interactions in that window are silently ignored. */
export async function goto(page: Page, path: string) {
  await page.goto(path);
  await page.locator("body[data-hydrated]").waitFor({ state: "attached" });
}

/** Rows of the main listing table (not tables inside the detail card). */
export function tableRows(page: Page): Locator {
  return page.locator("div.overflow-auto > table > tbody > tr");
}

/** Types via real keystrokes — `fill()` doesn't reliably trigger Leptos's `on:input`. */
export async function search(page: Page, placeholder: string, text: string) {
  const input = page.getByPlaceholder(placeholder);
  await input.clear();
  await input.pressSequentially(text);
}

export function detailCard(page: Page): Locator {
  return page.locator(".card", { has: page.locator(".card-title") });
}

export async function applyFilters(
  page: Page,
  configure: (dialog: Locator) => Promise<void>,
) {
  await page.getByRole("button", { name: /^Filters/ }).click();
  const dialog = page.locator("dialog[open]");
  await dialog.waitFor();
  await configure(dialog);
  await dialog.getByRole("button", { name: "Save" }).click();
}

/** Header cells located via CSS — Chromium doesn't expose a columnheader role for them. */
export function columnHeader(page: Page, label: string): Locator {
  return page
    .locator("div.overflow-auto > table > thead th")
    .filter({ hasText: label });
}
