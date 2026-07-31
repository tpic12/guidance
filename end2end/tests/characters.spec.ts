import { test, expect, type Page } from "@playwright/test";
import { goto, search } from "./helpers";

// Characters persist in e2e.db across runs, so every test creates a
// uniquely-named character and deletes it before finishing — no global
// count assertions.
const uniqueName = (base: string) => `${base} ${Date.now()}`;

async function next(page: Page) {
  await page.getByRole("button", { name: "Next" }).click();
}

// The wizard's tab bar is freely navigable — clicking any unlocked tab jumps
// straight there, no need to pass through the steps in between.
async function goToTab(page: Page, name: string) {
  await page.getByRole("tab", { name, exact: true }).click();
}

// Level lives on the Class step now (one stepper per added class, not a
// single Basics-step select) — clicking a class's own "+" (level - 1) times
// takes it from its default of 1 up to the target level.
async function setClassLevel(page: Page, classNamePattern: RegExp, level: number) {
  const row = page.locator("div.class-band").filter({ hasText: classNamePattern });
  const plus = row.getByRole("button", { name: "+" });
  for (let i = 1; i < level; i++) {
    await plus.click();
  }
}

async function deleteCharacter(page: Page, name: string) {
  await goto(page, "/characters");
  const card = page.locator(".card").filter({ hasText: name });
  await card.getByRole("button", { name: "Delete" }).click();
  await page.locator("dialog[open]").getByRole("button", { name: "Delete" }).click();
  await expect(card).toHaveCount(0);
}

// An ASI slot's Feat choice replaced its old plain `<select>` of feat
// options with a hover-to-preview/click-to-select list (mirroring Species
// and Background) — this switches the slot to "Feat" and clicks the named
// row instead of selecting an `<option>` by label.
async function pickAsiFeat(page: Page, levelLabel: string, featName: string) {
  const slot = page.locator("div.rounded-box").filter({ hasText: levelLabel });
  await slot.locator("select").first().selectOption("feat");
  await slot.getByRole("button", { name: featName }).click();
  return slot;
}

test("the wizard builds a level 5 warrior and renders its sheet", async ({ page }) => {
  const name = uniqueName("Wizard Flow Hero");
  await goto(page, "/characters");
  await page.getByRole("link", { name: "New Character" }).click();
  await expect(page).toHaveURL("/characters/new");

  // Basics
  await search(page, "e.g. Brenna Ironquill", name);
  await next(page);

  // Class + subclass (Fake Fist unlocks at 3, and is required once unlocked)
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await setClassLevel(page, /Fake Warrior/, 5);
  await page.getByRole("button", { name: /Fake Fist/ }).click();
  await next(page); // class -> optional features

  // Optional Features: Fake Fist (unlocked at 3) grants a second Fighting
  // Style pick on top of Fake Warrior's own class-level one (2 total), plus
  // 2 Maneuvers — both quotas exactly match this fixture set's pool sizes.
  // This step now comes right after Class/subclass, before the rest of the
  // build, since it goes hand-in-hand with that choice.
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await page.getByRole("button", { name: "Fake Guard Stance" }).click();
  await page.getByRole("button", { name: "Fake Deep Cut" }).click();
  await page.getByRole("button", { name: "Fake Trip Attack" }).click();
  await next(page); // optional features -> species

  // Species, background
  await page.getByRole("button", { name: /Fake Skyfolk/ }).click();
  await next(page);
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page);

  // Skills: Warrior offers "choose 2 from Athletics/Intimidation/Survival",
  // but Wanderer already fixed-grants Athletics+Survival, so both slots fall
  // back to the full skill list (skill_slots' collision handling) — picking
  // "Perception" (outside Warrior's original 3 options) proves the fallback
  // actually opened up, rather than the requirement silently shrinking.
  // The pool depends on two independently-resolving resources
  // (class_detail/background_detail), so asserting the value actually stuck
  // (not just firing selectOption) guards against a selection landing
  // during a transient recompute and getting reset moments later.
  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  await expect(skillSlot1.locator("select")).toHaveValue("intimidation");
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await expect(skillSlot2.locator("select")).toHaveValue("perception");
  await next(page); // skills -> abilities

  // Abilities: default standard array assignment is already valid
  await next(page); // abilities -> spells

  // Spells: Fake Warrior is a non-caster itself, but its Fake Fist subclass
  // grants third-caster spellcasting from level 3 on (Eldritch Knight-style),
  // restricted to the subclass-linked pool. At level 5 that's 2 cantrips
  // (both linked, so exactly enough) and 1 leveled spell — the fixture's
  // known-spells table wants more, but max spell level 1 only admits Fake
  // Whisper, so the required count clamps down to the pool's actual size.
  await page.getByRole("button", { name: "Fake Bolt" }).click();
  await page.getByRole("button", { name: "Fake Shimmer" }).click();
  await page.getByRole("button", { name: "Fake Whisper" }).click();
  await next(page); // spells -> asi

  // Level 4 ASI slot -> feat
  await pickAsiFeat(page, "Level 4 improvement", "Fake Brawler");
  await next(page);

  // Review: d10 at level 5 with Con 13 (+1) = 10 + 4*6 + 5 = 39 HP
  await expect(page.getByText("HP: 39")).toBeVisible();
  await page.getByRole("button", { name: "Save Character" }).click();

  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);
  await expect(page.locator("h1")).toHaveText(name);
  await expect(
    page.getByText("Level 5 Fake Warrior · Fake Fist · Fake Skyfolk · Fake Wanderer"),
  ).toBeVisible();
  await expect(page.getByText("HP 39")).toBeVisible();
  // Str 15 (+2) with proficiency (+3) from the warrior's save list.
  await expect(page.getByText("Strength +5")).toBeVisible();
  await expect(page.getByText("Fake Brawler (TBK)")).toBeVisible();
  await expect(page.getByText("Fist of Fakery (level 3)")).toBeVisible();
  await expect(page.getByText("Optional Features")).toBeVisible();
  await expect(page.getByText(/Fake Weapon Focus/)).toBeVisible();
  await expect(page.getByText(/Fake Guard Stance/)).toBeVisible();
  await expect(page.getByText(/Fake Deep Cut/)).toBeVisible();
  await expect(page.getByText(/Fake Trip Attack/)).toBeVisible();

  await deleteCharacter(page, name);
});

test("saving a multiclass build doesn't reject an optional feature that has no class-specific prerequisite", async ({ page }) => {
  // Regression coverage: "Fake Weapon Focus" (FS:F/FS:R, no `prerequisite` at
  // all) is only offered under Fake Warrior's own Fighting Style slot, but
  // `is_eligible` reads it as eligible for *any* class's `EligibilityContext`
  // since it has no class-specific prerequisite to fail — its real scoping
  // comes entirely from which class's own FeatureType pool it was fetched
  // into. `save_character` used to re-check every submitted choice against
  // every *globally* active FeatureType for every class (not just each
  // class's own), so once a second class (Fake Paladin, which grants no
  // optional features at all) existed on the character, it got checked
  // against Fighting Style too — found the pick "eligible" but its own quota
  // there is 0, and rejected an otherwise entirely valid save.
  const name = uniqueName("Multiclass Feature Check");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page); // basics -> class

  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await page.getByRole("button", { name: /Fake Paladin/ }).click();
  // Neither class's subclass unlocks until level 3, so both stay at their
  // default level 1 with no subclass required.
  await next(page); // class -> optional features

  // Fake Paladin grants no optional features at all, so this step still
  // renders as if single-class (no per-class sub-tab bar) — same as before
  // Fake Paladin was ever added.
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await next(page); // optional features -> species

  await page.getByRole("button", { name: /Fake Skyfolk/ }).click();
  await next(page);
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page);

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await next(page); // skills -> abilities
  await next(page); // abilities -> spells (Fake Paladin is a half-caster, but level 1 grants no slots yet)
  await next(page); // spells -> review (no ASI until level 4 for either class)

  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);
  await expect(page.getByText(/Level 1 Fake Warrior.*Level 1 Fake Paladin/)).toBeVisible();
  await expect(page.getByText(/Fake Weapon Focus/)).toBeVisible();

  await deleteCharacter(page, name);
});

test("an unfilled ASI slot blocks the Review checklist and disables Save", async ({ page }) => {
  // Regression coverage: unlocked_asi_levels resizes asi_choices with `None`
  // placeholders, and a slot left on "— unspent —" is still `Some` once a
  // kind is chosen but its inner id/codes are blank — step_complete(8) must
  // catch both cases, not just count slots.
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", "Blank ASI Check");
  await next(page); // basics -> class

  // Fake Fist unlocks at level 3, and is required once unlocked.
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await setClassLevel(page, /Fake Warrior/, 5);
  await page.getByRole("button", { name: /Fake Fist/ }).click();
  await next(page); // class -> optional features

  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await page.getByRole("button", { name: "Fake Guard Stance" }).click();
  await page.getByRole("button", { name: "Fake Deep Cut" }).click();
  await page.getByRole("button", { name: "Fake Trip Attack" }).click();
  await next(page); // optional features -> species

  await page.getByRole("button", { name: /Fake Skyfolk/ }).click();
  await next(page);
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page);

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await next(page); // skills -> abilities
  await next(page); // abilities -> spells

  await page.getByRole("button", { name: "Fake Bolt" }).click();
  await page.getByRole("button", { name: "Fake Shimmer" }).click();
  await page.getByRole("button", { name: "Fake Whisper" }).click();

  // Deliberately skip the Feats & ASIs tab entirely — jump straight to
  // Review with the level-4 ASI slot left blank.
  await goToTab(page, "Review");
  await expect(page.getByRole("button", { name: "Feats & ASIs" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Save Character" })).toBeDisabled();

  await page.getByRole("button", { name: "Feats & ASIs" }).click();
  await pickAsiFeat(page, "Level 4 improvement", "Fake Brawler");

  await goToTab(page, "Review");
  await expect(page.getByRole("button", { name: "Feats & ASIs" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Save Character" })).toBeEnabled();
});

test("editing a character reopens the wizard pre-filled", async ({ page }) => {
  const name = uniqueName("Edit Flow Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page);
  // Level 1: Fake Mage's "School of Fakery" subclass doesn't unlock until
  // level 2, so no subclass pick is required (or offered) yet.
  await page.getByRole("button", { name: /Fake Mage/ }).click();
  // Fake Mage's EI/PB progressions don't kick in until level 2/3, so at
  // level 1 Optional Features is locked and Next skips straight to Species.
  await next(page); // class -> species
  await page.getByRole("button", { name: /Fake Tunnelkin/ }).click();
  await next(page);
  await page.getByRole("button", { name: /^Fake Scholar \(TBK\)/ }).click();
  await next(page);

  // Skills: Mage offers "choose 2 from Arcana/Deception/History"; Scholar
  // fixed-grants History+Insight. History collides but Arcana+Deception
  // alone already satisfy the required count, so no full-list fallback here.
  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Arcana" });
  await expect(skillSlot1.locator("select")).toHaveValue("arcana");
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Deception" });
  await expect(skillSlot2.locator("select")).toHaveValue("deception");

  // Fake Mage's "Expertise" feature grants 2 expertise picks at level 1,
  // choosable from the full proficient set (fixed + just-chosen skills).
  const expertiseSlot1 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 1" });
  await expertiseSlot1.locator("select").selectOption({ label: "Arcana" });
  await expect(expertiseSlot1.locator("select")).toHaveValue("arcana");
  const expertiseSlot2 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 2" });
  await expertiseSlot2.locator("select").selectOption({ label: "Deception" });
  await expect(expertiseSlot2.locator("select")).toHaveValue("deception");
  await next(page); // skills -> abilities

  // Fake Tunnelkin grants Constitution +2 automatically, plus a choice of
  // any one ability +1 — pick Strength to complete the choice.
  await page.getByRole("button", { name: "Strength", exact: true }).click();
  await next(page); // abilities -> spells

  // Spells: Fake Mage is a known-caster. cantripProgression[0]=4 clamps to
  // this fixture's 2-cantrip pool; spellsKnownProgression[0]=2 clamps to the
  // single level-1 spell castable at character level 1 (Fake Whisper).
  await page.getByRole("button", { name: "Fake Bolt" }).click();
  await page.getByRole("button", { name: "Fake Shimmer" }).click();
  await page.getByRole("button", { name: "Fake Whisper" }).click();
  await next(page); // spells -> review (Fake Mage has no ASI feature at any level, Feats & ASIs is locked and skipped)
  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);

  await goto(page, "/characters");
  await page
    .locator(".card")
    .filter({ hasText: name })
    .getByRole("link", { name: "Edit" })
    .click();
  await expect(page.locator("h1")).toHaveText("Edit Character");
  await expect(page.getByLabel("Character name")).toHaveValue(name);

  // Skill/expertise choices are prefilled from the saved character and stay
  // valid across the level change, so no new interaction is needed on those
  // steps. Spells is the exception: spellsKnownProgression[2]=4 clamps to 2
  // level-1/2 spells now castable — one more than the level-1 save already
  // has, so the Spells step needs an extra pick.
  await next(page); // basics -> class
  await setClassLevel(page, /Fake Mage/, 3);

  // Level 3 crosses Fake Mage's subclass-unlock level (2), so a subclass is
  // now required before continuing — pick School of Fakery here.
  await page.getByRole("button", { name: /Fakery/ }).click();
  await next(page); // class -> optional features

  // Level 3 newly unlocks Fake Mage's class-level Eldritch-Invocation-like
  // and Pact-Boon-like progressions (both quota 1) — neither depends on a
  // subclass; Fake Sight/Pact of the Fake Chain are the only eligible
  // options at this point (before any spells are known).
  await page.getByRole("button", { name: "Fake Sight" }).click();
  await page.getByRole("button", { name: "Pact of the Fake Chain", exact: true }).click();
  await next(page); // optional features -> species
  await next(page); // species -> background
  await next(page); // background -> skills
  await next(page); // skills -> abilities
  await next(page); // abilities -> spells
  // School of Fakery (just picked) auto-grants Fake Whisper from level 2 on,
  // so it drops out of the pickable pool entirely (and the level-1 save's
  // pick of it is silently retracted) — the only pickable leveled spell left
  // at level 3 is Fake Ward, a level-2 spell.
  await page.getByRole("button", { name: "Fake Ward" }).click();
  await next(page); // spells -> review (Fake Mage has no ASI feature at any level, Feats & ASIs is locked and skipped)
  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);
  await expect(page.getByText(/Level 3 Fake Mage/)).toBeVisible();

  await deleteCharacter(page, name);
});

test("subclass-granted spells are excluded from picks and shown on the sheet", async ({ page }) => {
  // School of Fakery auto-grants Fake Whisper (the fixture's only level-1
  // spell) starting level 2 — always prepared, never a choice.
  const name = uniqueName("Domain Caster Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page);

  await page.getByRole("button", { name: /Fake Mage/ }).click();
  await setClassLevel(page, /Fake Mage/, 2);
  await page.getByRole("button", { name: /Fakery/ }).click();
  await next(page); // class -> optional features

  // Level 2 unlocks Fake Mage's Eldritch-Invocation-like progression (quota 1).
  await page.getByRole("button", { name: "Fake Sight" }).click();
  await next(page); // optional features -> species

  await page.getByRole("button", { name: /Fake Tunnelkin/ }).click();
  await next(page);
  await page.getByRole("button", { name: /^Fake Scholar \(TBK\)/ }).click();
  await next(page);

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Arcana" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Deception" });
  const expertiseSlot1 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 1" });
  await expertiseSlot1.locator("select").selectOption({ label: "Arcana" });
  const expertiseSlot2 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 2" });
  await expertiseSlot2.locator("select").selectOption({ label: "Deception" });
  await next(page); // skills -> abilities

  // Fake Tunnelkin grants Constitution +2 automatically, plus a choice of
  // any one ability +1 — pick Strength to complete the choice.
  await page.getByRole("button", { name: "Strength", exact: true }).click();
  await next(page); // abilities -> spells

  // Fake Whisper is excluded from the pickable pool entirely (it's the only
  // level-1 spell in the fixture set), so no "Level 1" leveled-spell section
  // renders at all — just the info panel and the still-pickable cantrips.
  await expect(page.getByText(/automatically gives you these.*Fake Whisper/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Fake Whisper" })).toHaveCount(0);
  await expect(page.locator("h4").filter({ hasText: "Level 1" })).toHaveCount(0);

  await page.getByRole("button", { name: "Fake Bolt" }).click();
  await page.getByRole("button", { name: "Fake Shimmer" }).click();
  await next(page); // spells -> review (Fake Mage has no ASI feature at any level, Feats & ASIs is locked and skipped)
  await page.getByRole("button", { name: "Save Character" }).click();

  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);
  await expect(page.getByText("Granted spells (always prepared):")).toBeVisible();
  await expect(page.getByText("Fake Whisper (L1")).toBeVisible();
  await expect(page.getByText("Optional Features")).toBeVisible();
  await expect(page.getByText(/Fake Sight/)).toBeVisible();

  await deleteCharacter(page, name);
});

test("pact-gated invocations become eligible only once the matching pact is chosen", async ({ page }) => {
  const name = uniqueName("Pact Gated Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page);

  // Level 3 crosses Fake Mage's subclass-unlock level (2), so a subclass
  // must be picked here even though this test is really about the class-
  // level Pact Boon/Invocation cross-check, not the subclass itself.
  await page.getByRole("button", { name: /Fake Mage/ }).click();
  await setClassLevel(page, /Fake Mage/, 3);
  await page.getByRole("button", { name: /Fakery/ }).click();
  await next(page); // class -> optional features

  // Fake Blade Bond (an Invocation-like pick requiring "Pact of the Fake
  // Chain") is shown up front alongside every other invocation, but disabled
  // until that pact is chosen — no longer filtered out of the pool entirely.
  const pactOfTheChain = page.getByRole("button", { name: "Pact of the Fake Chain", exact: true });
  const bladeBond = page.getByRole("button", { name: "Fake Blade Bond" });
  await expect(bladeBond).toHaveCount(1);
  await expect(bladeBond).toBeDisabled();
  await pactOfTheChain.click();
  await expect(bladeBond).toBeEnabled();
  await bladeBond.click();

  // Un-picking the pact disables Fake Blade Bond again and retracts the
  // pick (it's no longer eligible), not just its toggle state — the same
  // silent-retention behavior a narrowed spell pool already gets.
  await pactOfTheChain.click();
  await expect(bladeBond).toBeDisabled();
  await expect(bladeBond).toHaveAttribute("aria-pressed", "false");

  // Re-pick a valid combination and finish this step.
  await pactOfTheChain.click();
  await bladeBond.click();
  await next(page); // optional features -> species

  await page.getByRole("button", { name: /Fake Tunnelkin/ }).click();
  await next(page);
  await page.getByRole("button", { name: /^Fake Scholar \(TBK\)/ }).click();
  await next(page);

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Arcana" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Deception" });
  const expertiseSlot1 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 1" });
  await expertiseSlot1.locator("select").selectOption({ label: "Arcana" });
  const expertiseSlot2 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 2" });
  await expertiseSlot2.locator("select").selectOption({ label: "Deception" });
  await next(page); // skills -> abilities

  // Fake Tunnelkin grants Constitution +2 automatically, plus a choice of
  // any one ability +1 — pick Strength to complete the choice.
  await page.getByRole("button", { name: "Strength", exact: true }).click();
  await next(page); // abilities -> spells

  // School of Fakery auto-grants Fake Whisper from level 2 on, so it's no
  // longer pickable here — only Fake Ward (level 2) is available.
  await page.getByRole("button", { name: "Fake Bolt" }).click();
  await page.getByRole("button", { name: "Fake Shimmer" }).click();
  await page.getByRole("button", { name: "Fake Ward" }).click();
  await next(page); // spells -> review (Fake Mage has no ASI feature at any level, Feats & ASIs is locked and skipped)
  await page.getByRole("button", { name: "Save Character" }).click();

  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);
  await expect(page.getByText("Optional Features")).toBeVisible();
  await expect(page.getByText(/Fake Blade Bond/)).toBeVisible();
  await expect(page.getByText(/Pact of the Fake Chain/)).toBeVisible();

  await deleteCharacter(page, name);
});

test("expertise naming a background-fixed skill survives editing", async ({ page }) => {
  // Regression coverage: expertise's "proficient" set is fixed (class ∪
  // background) ∪ skill_choices. On the edit/prefill path, class_detail and
  // background_detail resolve independently — if the expertise-clearing
  // effect only guarded on class readiness, it could run and wipe a
  // prefilled expertise pick that names a background-only fixed skill
  // (history/insight, granted by Fake Scholar, not Fake Mage) while
  // background_detail was still resolving.
  const name = uniqueName("Expertise Edit Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page);
  // Level 1: no subclass unlocked/required yet for Fake Mage.
  await page.getByRole("button", { name: /Fake Mage/ }).click();
  // Fake Mage's EI/PB progressions don't kick in until level 2/3, so at
  // level 1 Optional Features is locked and Next skips straight to Species.
  await next(page); // class -> species
  await page.getByRole("button", { name: /Fake Tunnelkin/ }).click();
  await next(page);
  await page.getByRole("button", { name: /^Fake Scholar \(TBK\)/ }).click();
  await next(page);

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Arcana" });
  await expect(skillSlot1.locator("select")).toHaveValue("arcana");
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Deception" });
  await expect(skillSlot2.locator("select")).toHaveValue("deception");

  // Expertise picks deliberately name Fake Scholar's background-fixed
  // skills (not Fake Mage's class-chosen ones) to isolate this path.
  const expertiseSlot1 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 1" });
  await expertiseSlot1.locator("select").selectOption({ label: "History" });
  await expect(expertiseSlot1.locator("select")).toHaveValue("history");
  const expertiseSlot2 = page.locator("div.rounded-box").filter({ hasText: "Expertise choice 2" });
  await expertiseSlot2.locator("select").selectOption({ label: "Insight" });
  await expect(expertiseSlot2.locator("select")).toHaveValue("insight");
  await next(page); // skills -> abilities

  // Fake Tunnelkin grants Constitution +2 automatically, plus a choice of
  // any one ability +1 — pick Strength to complete the choice.
  await page.getByRole("button", { name: "Strength", exact: true }).click();
  await next(page); // abilities -> spells

  // Spells: same clamped known-caster picks as the other Fake Mage tests.
  await page.getByRole("button", { name: "Fake Bolt" }).click();
  await page.getByRole("button", { name: "Fake Shimmer" }).click();
  await page.getByRole("button", { name: "Fake Whisper" }).click();
  await next(page); // spells -> review (Fake Mage has no ASI feature at any level, Feats & ASIs is locked and skipped)
  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);

  await goto(page, "/characters");
  await page
    .locator(".card")
    .filter({ hasText: name })
    .getByRole("link", { name: "Edit" })
    .click();
  await expect(page.locator("h1")).toHaveText("Edit Character");

  // Re-open the Skills step and assert both expertise picks survived —
  // this is what the background_ready() guard fix protects.
  await next(page); // basics -> class
  await next(page); // class -> species (Optional Features locked at level 1, skipped)
  await next(page); // species -> background
  await next(page); // background -> skills
  await expect(page.locator("div.rounded-box").filter({ hasText: "Expertise choice 1" }).locator("select")).toHaveValue("history");
  await expect(page.locator("div.rounded-box").filter({ hasText: "Expertise choice 2" }).locator("select")).toHaveValue("insight");

  await deleteCharacter(page, name);
});

test("tabs are freely navigable and Review flags what's still incomplete", async ({ page }) => {
  await goto(page, "/characters/new");

  // Jump straight to Review with nothing filled in — no gating blocks this.
  await goToTab(page, "Review");
  await expect(page.getByText("Still needs:")).toBeVisible();
  await expect(page.getByRole("button", { name: "Basics" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Class" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Save Character" })).toBeDisabled();

  // Clicking a checklist entry jumps straight to that tab.
  await page.getByRole("button", { name: "Basics" }).click();
  await expect(page.getByLabel("Character name")).toBeVisible();

  await search(page, "e.g. Brenna Ironquill", "Gate Check");
  await goToTab(page, "Review");
  await expect(page.getByRole("button", { name: "Basics" })).toHaveCount(0);
  // Class is still unpicked, so it still shows up.
  await expect(page.getByRole("button", { name: "Class" })).toBeVisible();
});

test("a subclass must be selected once it unlocks, per the Review checklist", async ({ page }) => {
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", "Subclass Gate Check");
  await goToTab(page, "Class");

  // Fake Fist unlocks at level 3, so at level 5 picking Fake Warrior alone
  // isn't enough — a subclass is required too, and Review flags it.
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await setClassLevel(page, /Fake Warrior/, 5);
  await goToTab(page, "Review");
  await expect(page.getByRole("button", { name: "Class" })).toBeVisible();

  await goToTab(page, "Class");
  await page.getByRole("button", { name: /Fake Fist/ }).click();
  await goToTab(page, "Review");
  await expect(page.getByRole("button", { name: "Class" })).toHaveCount(0);
});

test("each added class's subclass panel stays confined to its own row", async ({ page }) => {
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", "Class Grid Check");
  await next(page); // basics -> class

  // Both classes' subclasses happen to share the short name "Fakery" in this
  // fixture set — a real regression guard against one row's subclass panel
  // (or its highlighted selection) leaking into the other's once there are
  // two rows on screen at once.
  await page.getByRole("button", { name: /Fake Mage/ }).click();
  await setClassLevel(page, /Fake Mage/, 2); // Mage's subclass unlocks at 2
  await page.getByRole("button", { name: /Fake Paladin/ }).click();
  await setClassLevel(page, /Fake Paladin/, 3); // Paladin's subclass unlocks at 3

  const mageRow = page.locator("div.class-band").filter({ hasText: "Fake Mage" });
  const paladinRow = page.locator("div.class-band").filter({ hasText: "Fake Paladin" });
  const mageFakery = mageRow.getByRole("button", { name: /^Fakery/ });
  const paladinFakery = paladinRow.getByRole("button", { name: /^Fakery/ });

  // Each row offers exactly its own subclass button, not a shared/duplicated
  // one bleeding across rows.
  await expect(mageFakery).toHaveCount(1);
  await expect(paladinFakery).toHaveCount(1);
  await expect(mageFakery).not.toHaveClass(/btn-primary/);
  await expect(paladinFakery).not.toHaveClass(/btn-primary/);

  // Picking one row's subclass only highlights that row's own button.
  await mageFakery.click();
  await expect(mageFakery).toHaveClass(/btn-primary/);
  await expect(paladinFakery).not.toHaveClass(/btn-primary/);

  await paladinFakery.click();
  await expect(paladinFakery).toHaveClass(/btn-primary/);
  await expect(mageFakery).toHaveClass(/btn-primary/);
});

test("the optional features step boxes sections in an adaptive grid, highlights chip picks, and surfaces resource costs", async ({ page }) => {
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", "Optional Features Grid Check");
  await next(page); // basics -> class

  // Fake Warrior alone at level 1 (Fake Fist isn't unlocked/required until
  // level 3) grants exactly one optional-feature section (Fighting Style) —
  // the grid should render as a single, width-capped column rather than
  // stretching a lone section edge-to-edge.
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await next(page); // class -> optional features

  const grid = page.locator('div[class*="grid-cols-1"]');
  await expect(grid).toHaveClass(/max-w-xl/);
  await expect(grid).not.toHaveClass(/md:grid-cols-2/);

  // A single checklist next to a fixed-width detail panel leaves the panel
  // looking stranded in empty space, so it grows (flex-1) to fill the row
  // instead once there's only one list to show it next to.
  const panel = page.locator('div[class*="lg:sticky"]');
  await expect(panel).toHaveClass(/flex-1/);
  await expect(panel).not.toHaveClass(/lg:w-96/);

  const fightingStyleSection = page.locator("h3", { hasText: "Fighting Style" }).locator("..");
  await expect(fightingStyleSection).toHaveClass(/card/);
  await expect(fightingStyleSection).toHaveClass(/bg-base-200/);

  const weaponFocus = page.getByRole("button", { name: "Fake Weapon Focus" });
  await expect(weaponFocus).toHaveAttribute("aria-pressed", "false");
  await weaponFocus.click();
  await expect(weaponFocus).toHaveAttribute("aria-pressed", "true");
  await expect(weaponFocus).toHaveClass(/btn-primary/);

  // Raising the level to 5 and picking Fake Fist (unlocked at level 3, and
  // required once unlocked) adds a second section (Maneuver) — the grid
  // should widen to two columns, and the panel switches back to a
  // fixed-width sidebar since it now sits beside a wider, two-column list.
  await goToTab(page, "Class");
  await setClassLevel(page, /Fake Warrior/, 5);
  await page.getByRole("button", { name: /Fake Fist/ }).click();
  await next(page); // class -> optional features

  await expect(grid).toHaveClass(/md:grid-cols-2/);
  await expect(grid).not.toHaveClass(/max-w-xl/);
  await expect(panel).toHaveClass(/lg:w-96/);
  await expect(panel).not.toHaveClass(/flex-1/);

  // Fake Deep Cut and Fake Trip Attack both consume a "Fake Superiority Die"
  // in this fixture set — a resource cost previously dropped from the UI
  // entirely, now rendered as a subtitle under each option's name.
  await expect(page.getByText("Fake Superiority Die")).toHaveCount(2);
});

test("the Spells tab is locked for a non-caster", async ({ page }) => {
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", "Non Caster Check");
  await next(page);

  // Fake Warrior alone (Fake Fist isn't unlocked/required until level 3, and
  // this build stays at level 1) has no spellcasting at all.
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await next(page); // class -> optional features
  // Fake Warrior itself (independent of the Fake Fist subclass, not in play
  // here) grants a Fighting Style pick starting at level 1.
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await next(page); // optional features -> species

  await page.getByRole("button", { name: /Fake Skyfolk/ }).click();
  await next(page);
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page);

  // Spells is grayed out with a tooltip explaining why, rather than hidden
  // outright, and clicking it refuses to navigate away from the current tab.
  const spellsTab = page.getByRole("tab", { name: "Spells" });
  await expect(spellsTab).toHaveAttribute("title", /doesn't grant spellcasting/);
  await spellsTab.click();
  await expect(page.getByRole("heading", { name: "Skill Proficiencies" })).toBeVisible();

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });

  await goToTab(page, "Review");
  // Spells never shows up as something to finish, since it's locked.
  await expect(page.getByRole("button", { name: "Spells" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Save Character" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Save Character" })).toBeEnabled();
});

test("the species step groups reprinted species and applies the chosen source variant", async ({ page }) => {
  // Fake Duskkin is seeded twice (TBK and ZBK, with different traits/speed)
  // specifically to exercise the species step's grouping UI: one left-pane
  // row with a "(2)" count badge, expanding into a right pane of source
  // variants only once clicked.
  const name = uniqueName("Species Group Check");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page); // basics -> class

  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await next(page); // class -> optional features
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await next(page); // optional features -> species

  // Before the group is opened, only the collapsed "Fake Duskkin" row
  // exists — no variant stats are shown or selected yet. `exact: true`
  // matters here: Duskkin (TBK)'s ability line is otherwise a substring of
  // Fake Skyfolk's (also dex +2/wis +1), which stays visible the whole time
  // as one of the inline single-source rows.
  await expect(page.getByText("Dexterity +2, Wisdom +1 · Medium · 30 ft.", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: /^Fake Duskkin\b/ }).click();

  // Opening the group reveals both reprints' stats without picking either —
  // neither variant row is marked selected yet.
  await expect(page.getByText("Dexterity +2, Wisdom +1 · Medium · 30 ft.", { exact: true })).toBeVisible();
  await expect(page.getByText("Constitution +2 · Small · 25 ft.", { exact: true })).toBeVisible();
  const tbkVariant = page.getByRole("button", { name: /^Fake Duskkin \(TBK\)/ });
  await expect(tbkVariant).not.toHaveClass(/menu-active/);

  await tbkVariant.click();
  await expect(tbkVariant).toHaveClass(/menu-active/);
  await next(page); // species -> background

  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page); // background -> skills

  // Same skill fallback as the other Warrior+Wanderer tests: Wanderer's
  // fixed Athletics+Survival grants collide with Warrior's own options, so
  // both slots fall back to the full skill list.
  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await next(page); // skills -> abilities
  // Spells (non-caster) and Feats & ASIs (no ASI until level 4) are both
  // locked, so Next skips both in one jump straight to Review.
  await next(page); // abilities -> review
  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);

  // Proof the TBK variant (not ZBK) actually landed on the saved character,
  // not just that its row looked highlighted in the picker: TBK's own trait
  // (Dust Step) and speed (30 ft., no darkvision) show up on the sheet,
  // where the ZBK reprint's Grit Skin/25 ft./darkvision 60 would otherwise.
  await expect(page.getByText("Speed 30 ft.")).toBeVisible();
  await expect(page.getByText(/Darkvision/)).toHaveCount(0);
  await expect(page.getByText("Fake Duskkin Traits")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Dust Step" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Grit Skin" })).toHaveCount(0);

  await deleteCharacter(page, name);
});

test("the Optional Features, Species, and Background steps show hover/click details in a right-side panel", async ({ page }) => {
  const name = uniqueName("Detail Panel Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page); // basics -> class
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await next(page); // class -> optional features

  // Optional Features: hovering a checklist option previews it on the
  // right without toggling it; clicking both toggles the pick and updates
  // the preview to match.
  await expect(page.getByText("Hover or select an option to see its details.")).toBeVisible();
  const weaponFocus = page.getByRole("button", { name: "Fake Weapon Focus" });
  await weaponFocus.hover();
  await expect(page.getByRole("heading", { name: "Fake Weapon Focus" })).toBeVisible();
  await expect(weaponFocus).toHaveAttribute("aria-pressed", "false");
  await weaponFocus.click();
  await expect(weaponFocus).toHaveAttribute("aria-pressed", "true");

  // Unlike the single-pick Species/Background lists below, this is a
  // multi-select checklist — moving the mouse off the list does NOT revert
  // the preview to a placeholder or the last commit; it just stays wherever
  // it was last hovered/clicked, matching the Spells step's behavior.
  await page.getByRole("heading", { name: "Optional Features" }).hover();
  await expect(page.getByRole("heading", { name: "Fake Weapon Focus" })).toBeVisible();

  await next(page); // optional features -> species

  // Species: hovering a single-variant species previews its detail card on
  // the right without committing a pick; clicking commits it too.
  const speciesRow = page.getByRole("button", { name: /Fake Skyfolk/ });
  await speciesRow.hover();
  await expect(page.getByRole("heading", { name: /Fake Skyfolk/ })).toBeVisible();
  await expect(speciesRow).not.toHaveClass(/menu-active/);
  await speciesRow.click();
  await expect(speciesRow).toHaveClass(/menu-active/);

  // Hovering a different option still swaps the preview without touching
  // the commit, and leaving the list entirely (hovering the step's own
  // heading, well outside it) reverts the panel to the committed pick
  // rather than leaving it stuck on the last-hovered option.
  await page.getByRole("button", { name: /Fake Tunnelkin/ }).hover();
  await expect(page.getByRole("heading", { name: /Fake Tunnelkin/ })).toBeVisible();
  await page.getByRole("heading", { name: "Species" }).hover();
  await expect(page.getByRole("heading", { name: /Fake Skyfolk/ })).toBeVisible();

  await next(page); // species -> background

  // Background: same hover-previews/click-commits/reverts-on-leave behavior.
  const backgroundRow = page.getByRole("button", { name: /Fake Wanderer/ });
  await backgroundRow.hover();
  await expect(page.getByRole("heading", { name: /Fake Wanderer/ })).toBeVisible();
  await expect(backgroundRow).not.toHaveClass(/menu-active/);
  await backgroundRow.click();
  await expect(backgroundRow).toHaveClass(/menu-active/);

  await page.getByRole("button", { name: /Fake Scholar/ }).hover();
  await expect(page.getByRole("heading", { name: /Fake Scholar/ })).toBeVisible();
  await page.getByRole("heading", { name: "Background" }).hover();
  await expect(page.getByRole("heading", { name: /Fake Wanderer/ })).toBeVisible();
});

test("the Feats & ASIs step shows hover/click feat details only once Feat is chosen", async ({ page }) => {
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", "ASI Detail Panel Check");
  await next(page); // basics -> class

  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await setClassLevel(page, /Fake Warrior/, 4);
  await next(page); // class -> optional features
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await next(page); // optional features -> species
  await page.getByRole("button", { name: /Fake Skyfolk/ }).click();
  await next(page); // species -> background
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page); // background -> skills

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await next(page); // skills -> abilities
  // Fake Warrior alone (Fake Fist isn't in play here) is a non-caster, so
  // Spells is locked and Next skips straight to Feats & ASIs.
  await next(page); // abilities -> asi

  const slot = page.locator("div.rounded-box").filter({ hasText: "Level 4 improvement" });

  // No feat list/panel exists until the slot's kind is switched to "Feat" —
  // this hover-preview UI is deliberately scoped to that one sub-choice.
  await expect(slot.getByRole("list")).toHaveCount(0);
  await slot.locator("select").first().selectOption("abilities");
  await expect(slot.getByRole("list")).toHaveCount(0);

  await slot.locator("select").first().selectOption("feat");
  const brawlerRow = slot.getByRole("button", { name: "Fake Brawler" });
  await brawlerRow.hover();
  await expect(page.getByRole("heading", { name: "Fake Brawler" })).toBeVisible();
  await expect(brawlerRow).not.toHaveClass(/menu-active/);
  await brawlerRow.click();
  await expect(brawlerRow).toHaveClass(/menu-active/);

  // Hovering a different feat swaps the preview without touching the
  // commit, and leaving the list (hovering the slot's own label, outside
  // it) reverts the panel to the committed pick — same as Species/Background.
  const vigilanceRow = slot.getByRole("button", { name: "Fake Vigilance" });
  await vigilanceRow.hover();
  await expect(page.getByRole("heading", { name: "Fake Vigilance" })).toBeVisible();
  await slot.locator("span.font-semibold").hover();
  await expect(page.getByRole("heading", { name: "Fake Brawler" })).toBeVisible();

  // Switching back to Ability increases hides the feat list/panel entirely.
  await slot.locator("select").first().selectOption("abilities");
  await expect(slot.getByRole("list")).toHaveCount(0);
});

test("a level 10 Fake Paladin gets prepared spells, clamped to the fixture's spell pool", async ({ page }) => {
  const name = uniqueName("Prepared Caster Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page);

  // Oath of Fakery unlocks at level 3, so it's required at level 10.
  await page.getByRole("button", { name: /Fake Paladin/ }).click();
  await setClassLevel(page, /Fake Paladin/, 10);
  await page.getByRole("button", { name: /Fakery/ }).click();
  // Fake Paladin (and its Oath of Fakery subclass) has no
  // optionalfeatureProgression at all — Optional Features is locked and
  // Next skips straight to Species.
  await next(page); // class -> species

  await page.getByRole("button", { name: /Fake Skyfolk/ }).click();
  await next(page);

  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page);

  // Skills: Paladin offers "choose 2 from Athletics/Insight/Persuasion".
  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Insight" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Persuasion" });
  await next(page); // skills -> abilities

  await next(page); // abilities: default standard array is already valid -> spells

  // Spells: Fake Paladin is a half-caster with no cantripProgression, so it
  // gets zero cantrip slots (no "Cantrips" section renders at all). It's a
  // prepared caster (no spellsKnownProgression): level/2 (5) + cha mod (-1,
  // cha 8 default) = 4 wanted, clamped to the 3 level-1..3 spells this
  // fixture links to it (Fake Whisper/Fake Ward/Fake Blast). Leveled spells
  // are grouped under always-visible per-level headers (no accordion), so
  // every level's chips are reachable without expanding anything.
  await expect(page.getByRole("heading", { name: /Cantrips/ })).toHaveCount(0);
  // The count lives in a badge next to the "Spells" section heading (an h3,
  // distinct from the step's own "Spells" h2 title), not in the heading's
  // own text.
  const spellsCountBadge = page.locator("h3").filter({ hasText: "Spells" }).locator(".badge");
  await expect(spellsCountBadge).toHaveText("0/3");
  await page.getByRole("button", { name: "Fake Whisper" }).click();
  await page.getByRole("button", { name: "Fake Ward" }).click();
  await page.getByRole("button", { name: "Fake Blast" }).click();
  await expect(spellsCountBadge).toHaveText("3/3");

  // Regression coverage for the over-quota banner: dropping the prepared-
  // caster's ability score shrinks the required count. The wizard must NOT
  // silently drop one of the already-picked spells to match — it should
  // show a banner until the player deselects manually.
  await goToTab(page, "Abilities");
  await page.getByRole("tab", { name: "Manual" }).click();
  const chaScore = page.locator("div.rounded-box").filter({ hasText: "Charisma" }).locator("input");
  await chaScore.fill("3");
  await expect(chaScore).toHaveValue("3");
  await goToTab(page, "Spells");

  // cha 3 -> mod -4; level/2 (5) + -4 = 1 wanted, still within the 3-spell
  // pool, so required drops from 3 to 1 while 3 remain checked.
  await expect(spellsCountBadge).toHaveText("3/1");
  await expect(page.getByText("2 too many")).toBeVisible();

  // Deselect two of the three already-picked spells to reach the new,
  // lower required count.
  await page.getByRole("button", { name: "Fake Ward" }).click();
  await page.getByRole("button", { name: "Fake Blast" }).click();
  await expect(spellsCountBadge).toHaveText("1/1");
  await expect(page.getByText("too many")).toHaveCount(0);

  // Restore cha to 8 (back to the original required count of 3) and re-pick
  // the two spells removed above so the rest of this test's assertions
  // (which expect all three) still hold.
  await goToTab(page, "Abilities");
  await page.getByRole("tab", { name: "Standard Array" }).click();
  await goToTab(page, "Spells");
  await page.getByRole("button", { name: "Fake Ward" }).click();
  await page.getByRole("button", { name: "Fake Blast" }).click();
  await expect(spellsCountBadge).toHaveText("3/3");
  await next(page); // spells -> review (Fake Paladin has no ASI feature at any level, Feats & ASIs is locked and skipped)
  await page.getByRole("button", { name: "Save Character" }).click();

  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);
  await expect(page.getByText("Save DC 11 · Attack +3")).toBeVisible();
  await expect(page.getByText("Slots — Level 1: 4, Level 2: 3, Level 3: 2")).toBeVisible();
  await expect(page.getByText("Fake Whisper")).toBeVisible();
  await expect(page.getByText("Fake Ward")).toBeVisible();
  await expect(page.getByText("Fake Blast")).toBeVisible();

  await deleteCharacter(page, name);
});

test("a species bonus to the spellcasting ability changes the required spell count, alongside a subclass's always-prepared grant", async ({ page }) => {
  // Regression coverage: `spells_required` must derive its ability score the
  // same way `save_character` does — species bonus (and ASI) stacked on top
  // of the raw entered score, not the raw score alone. Using the raw score
  // let the wizard enable Next one spell short of what the server actually
  // requires, so Save failed with "Spell choices don't match what's
  // unlocked for this class and level" for any caster whose spellcasting
  // ability got bumped by a species trait (exactly what a real Druid +
  // Wisdom-boosting species/Circle subclass combination hit).
  const name = uniqueName("Species Bonus Caster Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page); // basics -> class

  // Oath of Fakery unlocks at level 3, so it's required at level 5.
  await page.getByRole("button", { name: /Fake Paladin/ }).click();
  await setClassLevel(page, /Fake Paladin/, 5);
  await page.getByRole("button", { name: /Fakery/ }).click();
  // Fake Paladin (and its Oath of Fakery subclass) has no
  // optionalfeatureProgression at all — Optional Features is locked and
  // Next skips straight to Species.
  await next(page); // class -> species

  // Fake Mimicborn grants Charisma +2 automatically (default standard array
  // Charisma is 8, a -1 modifier — this bumps it to 10, a +0 modifier). The
  // Strength/Dexterity choice grant doesn't touch Charisma either way.
  await page.getByRole("button", { name: /Fake Mimicborn/ }).click();
  await next(page); // species -> background
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page); // background -> skills

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Insight" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Persuasion" });
  await next(page); // skills -> abilities

  await page.getByRole("button", { name: "Strength", exact: true }).click();
  await next(page); // abilities -> spells

  // Oath of Fakery auto-grants Greater Fake Bolt starting level 3, always
  // prepared. It isn't even castable yet at level 5 (a half-caster's 4th-
  // level slots don't open until level 12), so it can never appear among
  // the pickable chips below regardless of this grant — this only confirms
  // the "always prepared" grant itself is live and surfaced to the player
  // alongside their own picks.
  await expect(page.getByText(/automatically gives you these.*Greater Fake Bolt/)).toBeVisible();

  // Half-caster level component at level 5 is 5/2 = 2 (2nd-level slots, and
  // so Fake Ward, aren't reachable until level 5). With the raw, un-bonused
  // Charisma 8 (-1 mod) the wizard used to compute 2 + -1 = 1 required — one
  // short of the server's real count, which derives the modifier from the
  // *final* (species-bonused) score: Charisma 10 (+0 mod) -> 2 + 0 = 2. Fake
  // Whisper (L1) and Fake Ward (L2) are the only spells reachable at this
  // level, so the required count is exactly 2, not clamped down by a
  // smaller pool — the discrepancy is directly observable rather than
  // masked away.
  const spellsCountBadge = page.locator("h3").filter({ hasText: "Spells" }).locator(".badge");
  await expect(spellsCountBadge).toHaveText("0/2");

  await page.getByRole("button", { name: "Fake Whisper" }).click();
  await expect(spellsCountBadge).toHaveText("1/2");
  await page.getByRole("button", { name: "Fake Ward" }).click();
  await expect(spellsCountBadge).toHaveText("2/2");
  await next(page); // spells -> review (Fake Paladin has no ASI feature at any level, Feats & ASIs is locked and skipped)
  await page.getByRole("button", { name: "Save Character" }).click();

  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);
  await expect(page.getByText("Level 5 Fake Paladin · Fakery · Fake Mimicborn · Fake Wanderer")).toBeVisible();
  await expect(page.getByText("Fake Whisper")).toBeVisible();
  await expect(page.getByText("Fake Ward")).toBeVisible();
  await expect(page.getByText("Granted spells (always prepared):")).toBeVisible();
  await expect(page.getByText("Greater Fake Bolt")).toBeVisible();

  await deleteCharacter(page, name);
});

test("the Standard Array tab assigns scores via click-to-place chips", async ({ page }) => {
  const name = uniqueName("Chip Assignment Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page); // basics -> class
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await next(page); // class -> optional features
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await next(page); // optional features -> species
  await page.getByRole("button", { name: /Fake Skyfolk/ }).click();
  await next(page); // species -> background
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page); // background -> skills

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await next(page); // skills -> abilities

  // Every ability's own value button is disabled until a pool chip is
  // armed — clicking one before arming anything is a no-op, not a swap.
  const strengthBox = page.locator("div.rounded-box").filter({ hasText: "Strength" });
  const strengthValue = strengthBox.getByRole("button");
  await expect(strengthValue).toBeDisabled();
  await expect(strengthValue).toHaveText("15");

  // Arm the chip currently on Charisma (value 8) and drop it onto
  // Strength — the two values swap: Strength becomes 8, Charisma becomes
  // 15, and the chip disarms (every value button disables again).
  await page.getByText("CHA", { exact: true }).locator("..").click();
  await expect(strengthValue).toBeEnabled();
  await strengthValue.click();
  await expect(strengthValue).toHaveText("8");
  const charismaBox = page.locator("div.rounded-box").filter({ hasText: "Charisma" });
  await expect(charismaBox.getByRole("button")).toHaveText("15");
  await expect(strengthValue).toBeDisabled();

  // Spells (non-caster) and Feats & ASIs (no ASI until level 4) are both
  // locked, so Next skips both in one jump straight to Review.
  await next(page); // abilities -> review
  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);

  // Strength 8 (-1 mod) with Fighter's Strength save proficiency (+2 at
  // level 1) = +1 — proof the swap actually persisted, not just the UI.
  await expect(page.getByText("Strength +1")).toBeVisible();

  await deleteCharacter(page, name);
});

test("species ability bonuses auto-apply, including a Choose grant picker", async ({ page }) => {
  const name = uniqueName("Species Bonus Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page); // basics -> class
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await next(page); // class -> optional features
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await next(page); // optional features -> species

  // Fake Mimicborn grants Charisma +2 automatically, plus a choice of
  // Strength or Dexterity +1 — a restricted `from` set, unlike Fake
  // Tunnelkin's "choose any one of six" used elsewhere in this file.
  await page.getByRole("button", { name: /Fake Mimicborn/ }).click();
  await next(page); // species -> background
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page); // background -> skills

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await next(page); // skills -> abilities

  // The fixed Charisma +2 applies with no interaction: standard array's
  // default Charisma of 8 becomes 10, a +0 modifier instead of -1.
  const charismaBox = page.locator("div.rounded-box").filter({ hasText: "Charisma" });
  await expect(charismaBox).toContainText("+0");
  await expect(charismaBox).toContainText("+2 species");

  // The Choose grant is restricted to Strength/Dexterity only — Constitution
  // isn't offered, unlike Fake Tunnelkin's free choice of any ability.
  await expect(page.getByRole("button", { name: "Constitution", exact: true })).toHaveCount(0);
  const strengthChoice = page.getByRole("button", { name: "Strength", exact: true });
  await expect(strengthChoice).not.toHaveClass(/btn-primary/);
  await strengthChoice.click();
  await expect(strengthChoice).toHaveClass(/btn-primary/);

  // Strength 15 (standard array default) + 1 species = 16, a +3 modifier.
  const strengthBox = page.locator("div.rounded-box").filter({ hasText: "Strength" });
  await expect(strengthBox).toContainText("+3");
  await expect(strengthBox).toContainText("+1 species");

  // Spells (non-caster) and Feats & ASIs (no ASI until level 4) are both
  // locked, so Next skips both in one jump straight to Review.
  await next(page); // abilities -> review

  // The Review step's own ability grid must already reflect the bonused
  // scores, matching the Abilities step's live preview.
  await expect(page.getByText("Level 1 Fake Warrior · Fake Mimicborn · Fake Wanderer")).toBeVisible();
  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);

  // The saved sheet must match what the wizard showed, not the raw base
  // scores — the main regression risk of threading species through
  // final_abilities(). Strength is a proficient save (+3 mod + 2 prof);
  // Charisma isn't proficient, so its line is the bare +0 mod.
  await expect(page.getByText("Strength +5")).toBeVisible();
  await expect(page.getByText("Charisma +0")).toBeVisible();

  await deleteCharacter(page, name);
});

test("a custom ability bonus overrides the species' own grant and survives an edit round-trip", async ({
  page,
}) => {
  const name = uniqueName("Custom Bonus Hero");
  await goto(page, "/characters/new");

  await search(page, "e.g. Brenna Ironquill", name);
  await next(page); // basics -> class
  await page.getByRole("button", { name: /Fake Warrior/ }).click();
  await next(page); // class -> optional features
  await page.getByRole("button", { name: "Fake Weapon Focus" }).click();
  await next(page); // optional features -> species

  // Fake Mimicborn grants a fixed Charisma +2 plus a Strength/Dexterity
  // Choose grant — picking Custom below must apply neither.
  await page.getByRole("button", { name: /Fake Mimicborn/ }).click();
  await next(page); // species -> background
  await page.getByRole("button", { name: /Fake Wanderer/ }).click();
  await next(page); // background -> skills

  const skillSlot1 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 1" });
  await skillSlot1.locator("select").selectOption({ label: "Intimidation" });
  const skillSlot2 = page.locator("div.rounded-box").filter({ hasText: "Skill choice 2" });
  await skillSlot2.locator("select").selectOption({ label: "Perception" });
  await next(page); // skills -> abilities

  await page.getByRole("tab", { name: "Custom bonus" }).click();
  // Switching to Custom pre-fills Strength (+2) / Dexterity (+1) without
  // forcing an interaction first.

  const customBonus = page
    .locator("div.flex.flex-col.gap-1")
    .filter({ hasText: "Pick two different abilities" });
  // Switching to Custom defaults to Strength (+2) / Dexterity (+1), so the
  // +1 slot is changed first — otherwise Dexterity would still be disabled
  // in the +2 slot as "already picked in the sibling slot".
  await customBonus.locator("select").nth(1).selectOption({ label: "Wisdom" }); // +1
  await customBonus.locator("select").nth(0).selectOption({ label: "Dexterity" }); // +2

  // Custom mode must ignore Fake Mimicborn's own +2 Charisma grant entirely
  // — no bonus badge, base 8 stays an untouched -1 modifier.
  const charismaBox = page.locator("div.rounded-box").filter({ hasText: "Charisma" });
  await expect(charismaBox).not.toContainText("species");
  await expect(charismaBox).toContainText("-1");
  // Standard array's Dexterity 14 + 2 custom = 16 (+3 mod); Wisdom 10 + 1
  // custom = 11 (+0 mod) — both carry a bonus badge (the "species" label is
  // shared with the species path; only the source of the diff differs).
  const dexterityBox = page.locator("div.rounded-box").filter({ hasText: "Dexterity" });
  await expect(dexterityBox).toContainText("+3");
  await expect(dexterityBox).toContainText("+2 species");
  const wisdomBox = page.locator("div.rounded-box").filter({ hasText: "Wisdom" });
  await expect(wisdomBox).toContainText("+1 species");

  // Spells (non-caster) and Feats & ASIs (no ASI until level 4) are both
  // locked, so Next skips both in one jump straight to Review.
  await next(page); // abilities -> review
  await page.getByRole("button", { name: "Save Character" }).click();
  await page.waitForURL(/\/characters\/(?!new$)[0-9a-f-]+$/);

  // Charisma is untouched (base 8, -1 mod); Dexterity is standard array's
  // 14 + 2 custom = 16, a +3 mod.
  await expect(page.getByText("Charisma -1")).toBeVisible();
  await expect(page.getByText("Dexterity +3")).toBeVisible();

  await goto(page, "/characters");
  await page.locator(".card").filter({ hasText: name }).getByRole("link", { name: "Edit" }).click();
  await expect(page.locator("h1")).toHaveText("Edit Character");
  // Tabs are freely navigable, so jump straight to Abilities rather than
  // clicking through every step in between.
  await goToTab(page, "Abilities");

  // The Custom toggle and both picks must round-trip from the saved
  // character, not silently reset to Species mode.
  await expect(page.getByRole("tab", { name: "Custom bonus" })).toHaveClass(/tab-active/);
  const editedCustomBonus = page
    .locator("div.flex.flex-col.gap-1")
    .filter({ hasText: "Pick two different abilities" });
  await expect(editedCustomBonus.locator("select").nth(0)).toHaveValue("dex");
  await expect(editedCustomBonus.locator("select").nth(1)).toHaveValue("wis");

  await deleteCharacter(page, name);
});
