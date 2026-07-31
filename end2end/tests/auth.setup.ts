import { test as setup } from "@playwright/test";
import { goto } from "./helpers";

const authFile = "./.auth/admin.json";

setup("authenticate as the bootstrap admin", async ({ page }) => {
  await goto(page, "/login");

  await page.getByLabel("Username").pressSequentially("admin");
  await page.getByLabel("Password").pressSequentially("admin");
  await page.getByRole("button", { name: "Log in" }).click();

  await page.waitForURL("/");
  await page.context().storageState({ path: authFile });
});
