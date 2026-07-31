# Guidance — Technical Design Document

**Stack:** Rust, Leptos 0.8 (nightly), Axum, SQLite, Tailwind + daisyUI
**Purpose:** Self-hosted TTRPG toolkit for managing shared compendium and character data. One instance, one hoster, one or more groups/users sharing a compendium of imported/homebrew content.

This document captures the architectural decisions made so far and lays out a build order you can turn directly into tickets. Phases are sequential — each one is a real, demoable increment, and later phases lean on infrastructure built in earlier ones.

**Progress at a glance** (updated as phases land — see each phase's **Status** line for detail):

| Phase | Status |
| --- | --- |
| 0 — Auth/permissions | ✅ Done |
| 1 — Raw import pipeline | 🚧 Partial — transform pattern proven, `raw_imports` staging not built |
| 2 — Spell transform | ✅ Done |
| 3 — Compendium UI (spells) | ✅ Done |
| 4 — Real import endpoint | ⬜ Not started |
| 5 — Classes/backgrounds/species/feats/items | 🚧 Partial — classes, backgrounds, species, feats done; items not started |
| 6 — Character creation | 🚧 Partial — core wizard + skill proficiencies/expertise done; equipment, multiclass, spell selection not yet |
| 7 — Playable sheet + write-behind | ⬜ Not started |
| 8 — Source management UI | ⬜ Not started |
| 9 — Homebrew authoring | ⬜ Not scoped (no design yet beyond the permission flag) |

---

## 1. Core Architectural Principles

These are the rules everything else below follows. When a future decision feels ambiguous, come back to these.

1. **Three data layers, never collapsed.**
   - **Reference data** (spells, classes, items, backgrounds, species) — imported once, shared instance-wide, rarely mutated.
   - **Character instance data** — a player's _choices_, stored as foreign keys into reference data, never copies of rule text.
   - **Resolved/computed data** (AC, HP max, spell save DC, the full rendered sheet) — never stored, always derived at request time by joining layers 1 + 2.

2. **Store inputs, derive outputs.** If a number can be calculated from other stored fields, it is not a column.

3. **Foreign key over duplication.** If a value can change because of errata or a re-import, it must be a reference, not a copy.

4. **Two-stage import, always.** Raw source JSON is archived untouched first (`raw_imports`), then transformed into clean typed rows second. Never skip straight to clean rows — the raw archive is your replay/debug safety net and your no-redistribution boundary (see §4).

5. **Content is shared, ownership is attribution not access.** Any user on the instance can reference any imported content. `imported_by` / `source_id` fields answer "who's accountable for this," not "who can see this."

6. **Permissions are capability grants, not roles.** A `user_permissions` table of discrete flags (`import_sources`, `create_homebrew`, `manage_users`, `archive_sources`), not a hardcoded enum tier. One exception: `is_instance_owner` is a hard bootstrap escape hatch.

7. **Play state is write-behind, not save-on-click.** During active play, the in-memory signal is the source of truth. The DB is a lagging mirror, flushed on debounce/checkpoint/unload — the user never sees or needs a "save" button.

---

## 2. Data Model Reference

Full types are in the appendix. Summary of what lives where:

| Concern                   | Table(s)                                               | Mutation pattern                                                             |
| ------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------- |
| Users & auth              | `users`                                                | Rare, explicit                                                               |
| Permissions               | `user_permissions`                                     | Rare, explicit, admin-only                                                   |
| Raw imported source blobs | `raw_imports`                                          | Write-once, soft-archived, rarely purged                                     |
| Reference content         | `spells`, `classes`, `items`, `backgrounds`, `species` | Write via transform pipeline only, treated as read-only by the app otherwise |
| Class↔spell availability  | `class_spells`                                         | Written by transform pipeline                                                |
| Character build           | `characters`                                           | Explicit save, deliberate "builder" UX                                       |
| Character play state      | `character_state`                                      | Write-behind, debounced, versioned                                           |

---

## 3. Build Order

### Phase 0 — Foundations: users, auth, permissions skeleton — ✅ Done

_New — this wasn't discussed explicitly yet but everything downstream needs it._

**Status:** Shipped beyond the minimal ask — `tower-sessions` + argon2, full account settings UI (change username/password, per-user theme), and admin user/permission management, not just a bare login form. One naming deviation from this doc: the FK column is `user_id` throughout the codebase, not `owner_id` (see also Phase 6 note).

**Goal:** the minimum auth needed for `imported_by` / `owner_id` fields to mean something.

- `users` table + a basic login flow (session cookie is enough to start; don't build OAuth yet)
- Bootstrap: first user created becomes `is_instance_owner = true`
- `user_permissions` table + a `Permission` enum + a `require_permission!()` extractor/guard for Axum handlers
- No UI polish needed here — a login form and a way to see "logged in as X" is enough

**Done when:** you can log in as two different local users and an Axum handler can reject one of them based on a missing permission.

**Ticket candidates:**

- [x] `users` table + migration
- [x] Session-based login (login/logout — no public self-registration, by design; new users are admin-created)
- [x] Bootstrap first-user-is-owner logic
- [x] `user_permissions` table + `Permission` enum
- [x] Axum extractor that loads current user + checks a required permission

---

### Phase 1 — Raw import pipeline (dev CLI first, no UI) — 🚧 Partial

**Status:** The parse/transform pattern this phase set out to prove is built and working (`src/importer/`), but the two-stage `raw_imports` archive design (principle #4) was skipped — the current `src/bin/seed.rs` reads local fixture files and inserts straight into final tables, no raw blob is ever archived in the DB. This is the main gap before Phase 4 (real upload endpoint) can be built the way this doc intends.

**Goal:** prove the "user brings their own JSON, we archive it untouched" flow mechanically, without building any upload UI yet.

- `raw_imports` table
- `importer` crate (separate from `src/`, no web-framework dependencies) with `import_spells(raw_json, source_name, user_id, pool) -> ImportSummary`
- Dev-only binary `src/bin/import_cli.rs` that reads a local file (kept outside `src/`, e.g. `fixtures/` or a gitignored `data/` folder — never `include_str!`'d, never committed if it contains real sourcebook text) and calls `importer::import_spells`
- At this stage the "transform" step can be a no-op — just prove bytes go in and land in `raw_imports` correctly

**Done when:** `cargo run --bin import_cli -- --file data/spells.json --source-name "Test Pack"` inserts a row into `raw_imports` and prints a summary.

**Ticket candidates:**

- [ ] `raw_imports` table + migration
- [x] `importer` crate scaffold (as `src/importer/`, not a separate crate — parse/transform modules per content type)
- [ ] `store_raw_blob()` (no raw archive step exists yet)
- [x] Dev CLI binary (`src/bin/seed.rs`, reads `fixtures/`/`test-fixtures/` directly)

---

### Phase 2 — Spell transform (the hard part, isolated) — ✅ Done

**Goal:** turn archived raw JSON into clean `Spell` rows, fully unit-testable in isolation before any UI exists.

- Define the target `Spell` struct + `ContentBlock` enum (appendix §A)
- `parse.rs` — validate raw JSON shape into an intermediate struct
- `transform.rs`:
  - map `school` string → `School` enum
  - flatten `entries` prose (with `{@damage}` / `{@condition}` style tags) into `Vec<ContentBlock>`
  - resolve scaling (`scalingLevelDice` or equivalent) into `SpellScaling`
  - map damage/save fields into your existing enums (reuse `Ability` from the character model — don't redefine it)
- Hand-write 4–5 fixture spells yourself covering: a scaling cantrip, a leveled spell with a save + material component, a ritual, a concentration spell. **Do not copy real sourcebook text into these fixtures** — invented or SRD-only content only, since this is what will live in your test suite/repo long-term.
- Unit tests against those fixtures, not against a real 5000-row dataset yet

**Done when:** `cargo test -p importer` passes against your hand-written fixtures and produces correctly-shaped `Spell` values, including correctly parsed `ContentBlock`s.

**Ticket candidates:**

- [x] `Spell` + `ContentBlock` + `School`/`SpellScaling` types
- [x] `parse::parse_spell_array`
- [x] `transform::spells_from_parsed`
- [x] Entries → `ContentBlock` tag parser
- [x] Fixture file + unit tests
- [x] `spells` table + migration

---

### Phase 3 — Compendium UI: spell list + detail (first full-stack slice) — ✅ Done

**Goal:** the simplest possible full round trip — read-only, no mutation, no state sync complexity — to validate routing, components, and rendering conventions before multiplying by four more content types.

- Leptos routes: `path!("/compendium/spells")` (list) and `path!("/compendium/spells/:id")` (detail), using `<ParentRoute>`/`<Outlet />` per your routing conventions
- Spell list: daisyUI table, filterable by level/school
- Spell detail: renders `ContentBlock`s — this is where the dice-roll chip and condition-link components get built
- Manually seed `spells` rows (via the Phase 1/2 CLI pipeline) to have real data to render against

**Stop and sanity-check the whole pipeline here before continuing** — this is your checkpoint that raw → transform → stored → rendered actually works end to end.

**Ticket candidates:**

- [x] Spell list route + daisyUI table component
- [x] Spell detail route
- [x] `ContentBlock` renderer (text/dice/reference/list variants)
- [x] Basic level/school filter UI

---

### Phase 4 — Real import endpoint — ⬜ Not started

**Status:** No upload endpoint or UI exists — content only ever gets in via `seed.rs` reading local files. Depends on Phase 1's `raw_imports` staging landing first (or at least being decided against) to avoid building the endpoint against the wrong shape.

**Goal:** replace "dev CLI reads a file" with the actual production flow — same `importer` logic, different entry point.

- Axum handler `POST /api/sources` accepting multipart upload, gated by `Permission::ImportSources`
- Calls the same `importer::import_spells` function the CLI calls — no divergent logic
- Minimal upload UI (file input or textarea paste + source name field)
- Each content type (spells/classes/species/backgrounds/feats/items/...) is still imported as its own separate file/request — there's no single "book" upload (a 5etools book JSON is that source's readable chapter text, not compendium stat data; see the fixtures/books/ note below). Because of that, an instance can easily end up with some tables populated and others empty (e.g. spells imported, items never uploaded) with nothing surfacing it. Some signal — a dashboard/compendium empty-state, or a status line per content type ("Items: 0 records") — should tell an admin/hoster which content types still have zero rows, so a partially-seeded instance is obvious rather than silently missing whole categories.

**Done when:** a permitted user can paste/upload JSON through the actual app UI and see it show up in the compendium.

**Ticket candidates:**

- [ ] `POST /api/sources` handler (multipart)
- [ ] Permission gate on the route
- [ ] Upload form UI
- [ ] Per-content-type "has any data yet" indicator (dashboard banner or compendium empty-state) so an admin can tell which tables are still unseeded

---

### Phase 5 — Repeat the pattern: classes → items → backgrounds/species — 🚧 Partial

**Status:** Classes, backgrounds, species, feats, and optional features (none of the last three were originally listed in this phase's header, but were built alongside the others) are all done — transform, table, and compendium list/detail routes for each. **Items are not started**: `/compendium/items` (`src/pages/items.rs`) is still the original placeholder page from repo scaffolding, no `items` table or importer exists.

**Optional features scope note:** "optional features" (Eldritch Invocations, Metamagic, Maneuvers, Elemental Disciplines, Infusions, Runes, Fighting Styles, Pact Boons — 5etools' `optionalfeature` content type, distinguished by a `featureType` code) has both compendium browse (`src/models/optional_feature.rs`, `src/importer/{parse,transform}_optional_feature.rs`, `optional_features`/`optional_feature_types` tables, `src/pages/optional_features.rs`) and character-creator selection (see Phase 6's note below) shipped. Two things still deliberately out of scope:
- `PrerequisiteOption`'s `class_name`/`subclass_name` fields are plain names (+ optional source), not resolved `Class.id`/`Subclass.id` FKs — the raw data references subclasses by name only and frequently omits source, so there's no reliable way to pre-compute today's `Subclass.id` slug at import time. The character builder matches by name against the already-loaded `ClassDetail`/`Subclass` instead; see the doc comment on `PrerequisiteOption` for the full reasoning.
- `additionalSpells` and `optionalfeatureProgression` *nested on individual `optionalfeature` entries themselves* (e.g. Blessed Warrior granting 2 cleric cantrips, or Superior Technique granting a further Maneuver pick) — a different, still-unparsed mechanic from the class/subclass-level `optionalfeatureProgression` Phase 6 now consumes (a subclass/class granting picks of a `FeatureType`). Not modeled; the prose in `entries` still describes the effect in plain English, so nothing is lost for browsing.

**Goal:** apply the now-proven transform + compendium pattern to the remaining content types. Do classes next — character creation needs them most, and they're the most structurally complex (level progression tables, subclass features), so better to hit that complexity now while the pattern is fresh.

For each content type: transform function + fixtures + unit tests → table → list/detail route. Should go faster each time since the shell (routing, list/detail pattern, `ContentBlock` rendering) is reused, not rebuilt.

**Ticket candidates (×4, one set per content type):**

- [x] Classes — struct + transform + fixtures/tests + table + list/detail routes
- [x] Backgrounds — struct + transform + fixtures/tests + table + list/detail routes
- [x] Species — struct + transform + fixtures/tests + table + list/detail routes
- [x] Feats — struct + transform + fixtures/tests + table + list/detail routes (bonus content type, not in the original ×4)
- [x] Optional features — struct + transform + fixtures/tests + table + filter/detail (bonus content type; character-creator selection shipped separately, see Phase 6)
- [ ] Items — struct + transform + fixtures/tests + table + list/detail routes

---

### Phase 6 — Character creation — 🚧 Partial

**Status:** The core wizard shipped — species/class/background selection, ability scores (standard array/point buy/manual), level, ASI/feat choices at unlocked levels, skill proficiency + expertise selection sourced from both class and background, spell selection for casters (incl. subclass-granted casters and subclass-granted always-prepared spells), optional-feature selection (Invocations, Fighting Style, Metamagic, Maneuvers, Infusions, Runes, Arcane Shots, Elemental Disciplines, Pact Boon), save/edit/delete, and a resolved read-only sheet view (AC, HP max, proficiency bonus, saving throws, per-skill modifiers, initiative). Server-side validation was tightened after a code-review pass to reject out-of-range abilities and ASI/skill/expertise choices inconsistent with what the class+background actually unlock — a direct API call can no longer bypass the wizard's UI guardrails. The wizard's ten steps (Basics, Class, Optional Features, Species, Background, Skills, Abilities, Spells, Feats & ASIs, Review) are freely-navigable tabs rather than a linear gated sequence: every tab is clickable in any order, and Back/Next just move to the adjacent index with no completeness gating. A step that currently has nothing to pick — Optional Features with no active `FeatureType` progressions, Skills with no choice pools, Spells for a non-caster, Feats & ASIs with no unlocked ASI level — renders as a locked (grayed, tooltipped) tab instead of being hidden or blocking progress; each of those lock checks reuses the same "nothing to choose" predicate the step body itself already renders as an empty-state message. Review is the single point of full validation: it lists every applicable-but-incomplete step as a clickable jump-link ("Still needs: Class, Skills") and keeps Save disabled until that list is empty, alongside the existing `Character::validate()` check. Not yet done, and next up: **starting equipment selection** (blocked on items not existing yet). **Multiclassing** is now partially landed: `Character.class_id: String` has become `classes: Vec<ClassLevel>` (appendix §B), with every derive function in `models::character` (HP, ASI slots, spell slots/known counts, expertise, optional-feature quotas, skill proficiencies) and `save_character`'s validation rewritten to sum/aggregate across every entry rather than assuming exactly one — including 5e's actual multiclass spell-slot rule (full/half/third caster levels summed into the full-caster table, but *only* when 2+ distinct caster classes are present; a solo half/third caster still uses its own table directly, since the combined formula doesn't reproduce it) and multiclass-specific skill proficiencies (`Class.multiclass_proficiencies`, imported from 5etools' `multiclassing.proficienciesGained`, granted to every class after the first instead of that class's full `proficiencies`). What's *not* landed yet: the builder wizard's Class step still only lets a player pick one class — the UI to add/level/subclass a second (or third) class, and the per-class sub-tabs that Skills/Spells/Optional Features/Feats & ASIs will need once that's possible, is the remaining piece. Also note: the live `Character` struct uses `user_id`/`id: String` rather than this doc's `owner_id: Uuid`, and folds feats into `asi_choices: Vec<Option<AsiChoice>>` rather than a separate `feats: Vec<CharacterFeat>` — appendix §B is the target shape, not yet the actual one.

**Skill/proficiency simplification worth revisiting:** when a class's or background's skill choice collides with a fixed grant from the other source and its own option list can't satisfy its required count (e.g. a class offers "choose 2 from Athletics/Intimidation/Survival" but the background already fixed-grants two of those three), the wizard falls back to letting the player pick from the full 18-skill list rather than narrowing the requirement. Follow-ups this surfaces: (1) whether that fallback is actually rules-legal — 5e's real rule is a replacement proficiency, not necessarily any skill; (2) whether the master skill list (`models::skill::SKILLS`, currently a hardcoded Rust constant) — and eventually tools/languages, which will need the same choice/expertise treatment — belongs in a DB table instead; (3) if it becomes a table, homebrew authors will want to add their own custom skills/tools/languages (via `create_homebrew`) — that needs a table for them to append to, not just a hardcoded list.

**Subclass-granted spells: known gaps worth revisiting.** Auto-granted subclass spells (Cleric domain spells, Druid circle spells, ...) are imported from 5etools' `additionalSpells` block (`src/importer/transform_class.rs`'s `granted_spell_names_at_level`/`granted_spell_groups`), excluded from the wizard's pickable pool, and shown on the sheet — but two shapes are deliberately skipped rather than modeled, and neither has a UI path yet:
- A `{"choose": ...}` filter (e.g. Nature Domain's free Wisdom-based cantrip, Death/Arcana Domain's free cantrip/spell from another class's list) is a *dynamic* pick, not a fixed grant — it's silently dropped at import, so that free spell is never offered anywhere in the wizard. Fixing this needs the wizard to resolve the filter (level/class/school constraints) into its own small choice pool, similar to `skill_slots`' `Choose`/`Any` handling for skills.
- A `{"daily": {...}}` wrapper (seen on Warlock's Fathomless patron) marks a once-per-day-use grant rather than an always-active one; also dropped at import, with no other representation.
- Separately: a re-import that changes a class/subclass's `additionalSpells` (newly marking a spell as granted that wasn't before) will make `save_character`'s pool-membership re-validation reject an existing character's already-saved pick of that same spell until the player re-edits and deselects it — an acceptable consequence of deriving the pool at request time (per this doc's core principles), but worth a heads-up if re-imports are expected against instances with real characters already saved.

**Optional feature selection: known gaps worth revisiting.** `optionalfeatureProgression` (class/subclass-level "N picks of FeatureType X at level Y", normalized in `transform_class.rs`'s `dense_progression`/`optional_feature_progressions_from_raw` into `Class`/`Subclass.optional_feature_progressions`) drives the wizard's Optional Features step and `save_character`'s matching re-validation (`models::character::{optional_feature_quota, active_optional_feature_types}`, `models::optional_feature::{EligibilityContext, is_eligible}`). Two things not modeled:
- Way of the Four Elements' `required` key (auto-grants "Elemental Attunement" for free at level 3, on top of the normal 2 discipline picks) isn't parsed — a Four Elements Monk just spends one of their 2 level-3 picks on it manually instead, a minor rules-inaccuracy (1 fewer effective pick that level) rather than a crash or a missing feature.
- `ResourceCost` (`consumes` — Sorcery Points, Superiority Dice, an Arcane Shot use, ...) is selected but not tracked or spent anywhere — this app has no play-loop/resource-tracking state yet at all (that's Phase 7's `character_state` territory), so this ticket only covers *choosing* these features, not *using* them in play.

**Multiclassing: known gap worth revisiting.** 5e requires minimum ability scores to multiclass into or out of a class (e.g. 13 Strength for Fighter, 13 Wisdom for Cleric) — a fixed ~12-class SRD table with no current home in `Class` or any imported data. This isn't enforced anywhere yet: the builder lets a player add any class combination regardless of their ability scores. Deliberately deferred to a follow-up ticket rather than folded into the multiclassing work above, since it's a "permission to add a class" check rather than a "shape of a choice" one, and needs its own hardcoded table plus new UI messaging distinct from every other validation in this file.

**Goal:** mostly composition, not new infrastructure — a multi-step wizard querying the same compendium tables and reusing the same card/detail components already built.

- `characters` table (from the earlier `Character` design — appendix §B)
- Multi-step builder UI: class (+ subclass, + optional features) → species → background → skills/proficiencies → abilities → equipment → spells (if caster) → ASI/feats
- Each step queries compendium tables directly (`SELECT * FROM classes`, etc.) — this is why Phase 3–5 had to come first
- Explicit "save" UX here is fine — this is a deliberate build flow, not the play loop

**Ticket candidates:**

- [x] `characters` table + migration
- [x] Builder wizard shell + step routing
- [x] Species/class/background selection steps
- [x] Ability score assignment (standard array / point buy / manual)
- [x] Skill proficiency + expertise selection (sourced from class and background)
- [x] Spell selection step (conditional on caster class)
- [x] Optional feature selection step (Invocations/Metamagic/Maneuvers/Infusions/Runes/Arcane Shots/Fighting Styles/Pact Boon), prerequisite + pact-gated, class+subclass quota aggregation
- [x] Tabbed free-navigation wizard shell (locked/grayed tabs instead of linear gating, Review completeness checklist) — landed as a prerequisite for multiclassing
- [ ] Starting equipment selection (blocked on items not existing yet)
- [ ] Multiclassing (`classes: Vec<ClassLevel>` per appendix §B — currently single `class_id`)

---

### Phase 7 — Playable sheet + write-behind persistence — ⬜ Not started

**Status:** No `character_state` table, HP/temp-HP/death-save tracking, or write-behind persistence pattern exists yet — the sheet today only renders derived numbers (AC, HP max, saves) computed fresh from `characters`, with nothing stored for live play state. This introduces a pattern nothing else in the codebase has: an in-memory signal as source of truth with debounced/checkpointed flush to the DB, per appendix §D.

**Goal:** the sheet becomes usable at the table — HP, spell slots, inventory toggles, conditions — without prompting for saves.

- `character_state` table (appendix §C), separate from `characters`
- `PlayState` signal wrapper with debounce-with-generation-token flush (appendix §D)
- Checkpoint triggers: short rest, long rest, level-up, manual "end session" — force immediate flush, bypass debounce
- `beforeunload` + `sendBeacon` flush on tab close
- `PATCH /api/characters/:id/state` with `version` optimistic-concurrency check

**Ticket candidates:**

- [ ] `character_state` table + migration
- [ ] `PlayState` signal + debounce/flush logic
- [ ] Checkpoint action wiring (rest/level-up/end session)
- [ ] `sendBeacon` unload flush
- [ ] Versioned PATCH endpoint + conflict handling

---

### Phase 8 — Source management UI — ⬜ Not started

**Status:** `Permission::ArchiveSources` exists in the permission enum (and is covered by generic grant/revoke permission tests), but nothing archives, purges, or lists sources yet — this waits on Phase 1's `raw_imports` table existing to have anything to manage.

**Goal:** let permitted users see, archive, and purge imported sources — deliberately last, since the transform logic needed to stabilize first.

- List view of `raw_imports` (status, imported-by, content type, row counts)
- Archive action (soft — sets `status = 'archived'`, hides from compendium, blocks new references, existing character references keep resolving)
- Purge action (hard delete) — must first show a reference count ("N characters currently use content from this source") before allowing it
- Re-import / re-transform action, re-running the transform step against the still-archived raw blob

**Ticket candidates:**

- [ ] Source list view
- [ ] Archive action + compendium filtering to exclude archived sources
- [ ] Reference-count check
- [ ] Purge action (gated behind the warning)
- [ ] Re-transform action

---

### Phase 9 — Homebrew authoring — ⬜ Not scoped

**Status:** `/homebrew` (`src/pages/homebrew.rs`) is a placeholder page, and `Permission::CreateHomebrew` exists but is checked nowhere. Unlike every other phase above, this one has **no design in this document** beyond the permission flag — it was deliberately left for last, but "last" still needs real decisions before it's buildable, not just implementation time:

- Shared-instance-wide by default, or private-to-creator with an explicit publish step?
- Same compendium tables with an `is_homebrew`/`created_by` marker (reusing existing list/detail routes and character-creation queries for free), or separate homebrew-specific tables?
- Any review/approval gate before homebrew content is selectable in character creation, or is `create_homebrew` the only gate?

**Ticket candidates:**

- [ ] Resolve the three design questions above
- [ ] Data model (schema change or new tables, depending on the above)
- [ ] Authoring UI, starting with spells (most mature transform/compendium pattern to mirror)
- [ ] Wire `Permission::CreateHomebrew` to the authoring UI/endpoint

---

### Backlog / explicitly deferred

Don't build these until a phase above surfaces a real need:

- **Dual-ruleset support (2014 + 2024 side by side)** — build Phase 1–5 against one edition first; generalize `canonical_id`/`ruleset` once the pipeline shape is proven.
- **HP/combat event log** (audit trail of "took 8 fire damage") — additive on top of Phase 7's snapshot state, not a prerequisite for it.
- **Live multi-tab/DM sync (WebSockets)** — Phase 7's optimistic-concurrency PATCH is sufficient until this becomes an actual pain point.

---

## 4. Pre-production Readiness

Feature phases above ship the product; these are the deployment-infrastructure gaps between "runs on a dev machine" and "a hoster can `docker compose up` this in production." Not phase-gated — pick these up whenever the actual deployment need shows up, same philosophy as the backlog above. Goal: turn the "Docker Compose-deployable" promise (README, CLAUDE.md) into something real. Everything below builds on `Dockerfile.dev`/`compose.yaml` (dev-loop tooling only, see README) without being either of those files.

- **Production image** — a real multi-stage `Dockerfile`: a build stage (same toolchain as `Dockerfile.dev`) that compiles the release binary + WASM/CSS assets, then a slim runtime stage that copies out just the compiled binary and `target/site`, with no Rust/Node toolchain left in the final image. Should run as a non-root user.
- **Production `docker-compose.yml`** — separate from the dev `compose.yaml`. This is where a named volume for the SQLite file is actually the right call — persisting data across image redeploys is exactly the problem that pattern solves, once there's a real image being redeployed. (For the dev loop, a plain bind-mounted file remains correct — that's a different problem.)
- **`connect_dev_db()` naming/behavior** — currently named and documented as dev-only ("Connects to the local sqlite dev database"), with a silent fallback to `sqlite://dev.db` if `DATABASE_URL` is unset. Before this doubles as the production entrypoint's db connection, decide: rename it, and likely make `DATABASE_URL` required (no silent default) in a production context, so a misconfigured deploy fails loudly instead of quietly writing to a file named `dev.db`.
- **Backup guidance for the SQLite file** — single-file db on a volume; document a backup approach (`sqlite3 .backup`, or a volume snapshot cadence) as part of shipping this to a real hoster.
- **CI image build/publish** — extend `.github/workflows/ci.yml` with a job that builds and pushes the production image (e.g. to GHCR) once it exists, matching the existing build/unit-test/e2e job split.
- **`testcontainers`** — once the production image exists, revisit using `testcontainers-node` in the Playwright suite to boot the *shipped* container instead of the current `cargo leptos end-to-end` process-spawn, so e2e exercises what actually gets deployed. Not useful on the Rust unit-test side today — SQLite is embedded/in-process, so there's no service to containerize there unless a real networked dependency (Postgres, Redis, etc.) gets added later.

**Ticket candidates:**

- [ ] Multi-stage production `Dockerfile` (build stage + slim non-root runtime stage)
- [ ] Production `docker-compose.yml` with a named volume for the SQLite file
- [ ] Rename/harden `connect_dev_db()` for production use (require `DATABASE_URL`, no silent `dev.db` default)
- [ ] Document a SQLite backup approach
- [ ] CI job to build + publish the production image
- [ ] Point the Playwright suite at the shipped image via `testcontainers-node`, once the production image exists

---

## Appendix

### A. Spell shape (Phase 2)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spell {
    pub id: SpellId,
    pub canonical_id: SpellId,
    pub ruleset: Ruleset,
    pub name: String,
    pub level: u8,
    pub school: School,
    pub source_id: Uuid,              // FK -> raw_imports
    pub licensing: Licensing,         // { srd: bool, basic_rules: bool }
    pub casting_time: CastingTime,
    pub range: Range,
    pub area_of_effect: Option<AreaOfEffect>,
    pub components: Components,
    pub duration: Duration,           // includes concentration: bool
    pub ritual: bool,
    pub damage_types: Vec<DamageType>,
    pub saving_throw: Option<Ability>, // reused from character model
    pub scaling: Option<SpellScaling>,
    pub description: Vec<ContentBlock>,
    pub higher_level_description: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentBlock {
    Text(String),
    DiceRoll { formula: String, label: Option<String> },
    Reference { kind: RefKind, id: String, display: String },
    List(Vec<String>),
}
```

### B. Character build shape (Phase 6)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub species: RaceId,
    pub background: BackgroundId,
    pub classes: Vec<ClassLevel>,
    pub abilities: AbilityScores,
    pub proficiencies: Proficiencies,
    pub inventory: Vec<InventoryItem>,   // starting/build-time inventory
    pub spellcasting: Vec<ClassSpellcasting>,
    pub feats: Vec<CharacterFeat>,
    pub notes: CharacterNotes,
}
```

### C. Play state shape (Phase 7)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterState {
    pub hp: HitPoints,
    pub death_saves: DeathSaves,
    pub spell_slots: HashMap<u8, SlotState>,
    pub conditions: HashSet<Condition>,
    pub exhaustion_level: u8,
    pub inspiration: bool,
    pub inventory: Vec<InventoryItem>,   // live equipped/attuned/quantity state
    pub currency: Currency,
}
```

```sql
CREATE TABLE character_state (
    character_id TEXT PRIMARY KEY REFERENCES characters(id),
    state_json   TEXT NOT NULL,
    version      INTEGER NOT NULL DEFAULT 1,
    updated_at   TEXT NOT NULL
);
```

### D. Write-behind flush pattern (Phase 7)

```rust
impl PlayState {
    fn mutate(&self, f: impl FnOnce(&mut CharacterState)) {
        self.state.update(f);
        self.schedule_flush();
    }

    fn schedule_flush(&self) {
        let my_gen = self.flush_gen.get_untracked() + 1;
        self.flush_gen.set(my_gen);
        let (state, flush_gen) = (self.state, self.flush_gen);
        spawn_local(async move {
            TimeoutFuture::new(1800).await;
            if flush_gen.get_untracked() == my_gen {
                flush_now(state.get_untracked()).await;
            }
        });
    }

    fn flush_immediately(&self) {
        self.flush_gen.update(|g| *g += 1);
        let state = self.state;
        spawn_local(async move { flush_now(state.get_untracked()).await; });
    }
}
```

### E. Permissions (Phase 0)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ImportSources,
    CreateHomebrew,
    ArchiveSources,
    ManageUsers,
}
```

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    is_instance_owner BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL
);

CREATE TABLE user_permissions (
    user_id TEXT NOT NULL REFERENCES users(id),
    permission TEXT NOT NULL,
    granted_by TEXT REFERENCES users(id),
    granted_at TEXT NOT NULL,
    PRIMARY KEY (user_id, permission)
);
```
