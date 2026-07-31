# AGENTS.md

Guidance for AI agents (Claude Code, CI/CD, future automation) working on this codebase.

## Before Starting Any Task

1. **Read CLAUDE.md first** — it's the source of truth for structure, stack, conventions, and design system.
2. **Understand the architecture** — this is a dual-target Leptos app (WASM + server), with character data built on top of imported reference content. See the Architecture section below.

## Architecture Principles That Affect Agent Decisions

**Three data layers, never collapsed:**
- Reference content (spells, classes, items — imported from sources, shared across the instance)
- Character build choices (FKs into reference content, entered by players, never duplicated rule text)
- Resolved/derived data (AC, HP, final ability scores — computed at request time, never stored)

When extending the database schema or adding a model field, ask: **Is this input, derived, or reference?** If it's derived, compute it; if it's reference, foreign key it; never store what can be fetched or calculated.

**Play state vs. creation state:**
- Character *creation* is explicit and deliberate — pull from the compendium, save to DB, with full validation.
- Play state (active sheet during a session) is write-behind in-memory — the character sheet signal is source of truth; DB is a checkpointed mirror. This distinction matters for when you validate and persist data.


## Working Within the Stack

**Build requirements:** Rust nightly, wasm target (`wasm32-unknown-unknown`), `cargo-leptos`, and `tailwindcss` v4. See CLAUDE.md for current versions and setup steps. Development runs in Docker Compose (`docker compose up` for hot-reload).

### Code Style & Conventions

- **Format with `leptosfmt`, not `rustfmt`** — `Cargo.toml` has an override that makes this the default for this workspace.
- **No comments unless necessary** — only write one when it conveys a gotcha, constraint, or why a workaround exists. One line max; multi-paragraph belongs in commit messages or PRs.
- **Prefer daisyUI utilities** — don't hand-roll CSS. Signature classes (`.nav-tab`, `.ledger-tile`, `.stat-seal`) are deliberately sparse; use them sparingly.
- **Prefer derived over stored** — if a value is computable (ability modifier, HP max, proficiency bonus), derive it at request time, not insert time.
- **Foreign key over duplication** — if a value could change via errata or re-import (class name, spell description), reference it by ID, don't copy the text.

### Feature Gates

- **`ssr` feature gates server-side dependencies** — `sqlx`, `anyhow`, `tokio`, anything that doesn't compile to WASM. Follow this pattern for new server-only code.
- **Both WASM and SSR builds are checked in CI** — locally, run `cargo check --features ssr` and `cargo build --target wasm32-unknown-unknown` to verify both compile.

## Testing & Verification

### Unit Tests

- Add `*_tests.rs` files as child modules (symlinked for private access).
- Place them next to the code they test (e.g., `src/models/spell_tests.rs` for `src/models/spell.rs`).
- Test edge cases: dangling FKs (references to purged sources), empty lists, boundary conditions.

### End-to-End Tests

- Add `.spec.ts` files in `end2end/tests/` directory (follows [Playwright conventions](https://playwright.dev)).
- Test user flows, not implementation details (click buttons, fill forms, verify page renders, not "check internal state").
- Use `test-fixtures/` data to set up known state; the e2e suite runs against an ephemeral `e2e.db`.
- Before claiming a feature is done, run `cargo leptos end-to-end` and confirm the happy path and edge cases work.

### Manual Testing

- Start the dev server (`docker compose up`), open the app in a browser, and use the feature end-to-end.
- Test on the route you're adding to; test that existing routes still work (no regressions).
- The app builds both a WASM bundle and server binaries; both must work for the feature to be production-ready.

## Import Pipeline & Data Transformation

The import pipeline currently uses a seed script (`src/bin/seed.rs`) to load fixture data into reference tables. Key files:

- `fixtures/spells.seed.json` — mock spell data for testing
- Seed script wires imports into `spells` and related tables

The seed script is early-stage; if adding new content types (items, feats, backgrounds), wire them similarly and coordinate with the owner on evolving the design.

## Git & Branching

- **Branch naming**: `<prefix>/<short-description>` where `prefix` is `feat`, `fix`, or `eng`.
- **Commit messages**: clear, one-sentence summary of the change. Explain the "why" if it's non-obvious.
- **Always branch from `main`**, never commit directly to it.
- **One clear commit per logical change** — squash WIP commits before the PR is ready.

## Common Queries During Implementation

### "Where does X go?"

- New pages (routes): `src/pages/*.rs`, export from `src/pages/mod.rs`, register in `src/app.rs` `<ParentRoute>`.
- New shared UI components: `src/components/*.rs`, export from `src/components/mod.rs`.
- Reference data models (Spell, Class, etc.): `src/models/*.rs`, export from `src/models/mod.rs`.
- Server-only functions (DB queries, imports): inside a `#[server]` block in the page/component file, or in `src/db.rs` if shared across multiple routes.
- Database schemas: `migrations/` (SQLx auto-migrates via `sqlx::migrate!`).

### "Should I store this field?"

Ask: "Could this change after import, or could I compute it?"
- **If it can change** (errata, re-import, player edits to build) → reference or store the input only, not the derived value.
- **If it's computable** (modifiers, derived ability scores, max HP) → derive it at request time from stored inputs.
- **If it's reference data** (spell school, class name, background description) → store the ID, fetch the text on read.

### "How do I handle broken imports?"

- Import pipeline should **tolerate dangling FKs** (a purged source shouldn't break existing character sheets).
- `get_character_sheet` resolves a character against the compendium gracefully; if a linked class/species/background is gone, the sheet still renders with the data that's available.
- Add warnings/notices to the UI if critical data is missing, but don't crash.


---

**Last updated**: 2026-07-29
