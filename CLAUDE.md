# CLAUDE.md

Guardrails and design constraints for this repository.

**Guidance** is a self-hosted TTRPG toolkit for managing shared rulebook content and character data. For task tracking, use Vikunja (via the `vikunja` skill). For implementation patterns, see AGENTS.md. For repo structure and commands, see README.md.

## What NOT to do

**Data model:**
- ❌ Don't store derived values (HP max, ability modifiers, proficiency bonus). Compute them at request time from stored inputs.
- ❌ Don't duplicate rule text or reference data. Foreign key and fetch at read time. If a value could change via errata/re-import, reference it by ID, not text.
- ❌ Don't collapse the three data layers: reference content (imported/shared), character build inputs (FKs + player choices), derived/resolved data (computed on request).
- ❌ Don't break existing characters when a source is purged. Import pipeline must tolerate dangling FKs.

**Play state vs. creation state:**
- Character *creation* is explicit and deliberate (full validation, save to DB).
- Play state (active sheet during play) is write-behind in-memory (character sheet signal is source of truth; DB is a checkpointed mirror).

**Server-side code:**
- ❌ Don't add server-only dependencies to the root `Cargo.toml`. Gate them behind the `ssr` feature so the WASM build stays clean.
- ❌ Don't run `cargo fmt` bare — use `leptosfmt` instead (Cargo.toml override sets this, but double-check on new machines).
- ❌ Don't use the default URL-encoded codec for server fns with `Vec`/`Option` fields; it silently drops empty collections. Opt into `#[server(input = Json)]` when needed.

**Styling:**
- ❌ Don't invent new CSS classes for one-off treatments. Use daisyUI utilities (card, drawer, menu, navbar, btn, etc.) or reach for the signature classes: `.nav-tab`, `.ledger-tile`, `.stat-seal` (sparingly — it's a one-place flourish).
- ❌ Don't apply `.stat-seal` or index-code patterns to every number/card just because they're available. They're deliberately sparse to stay a signature.
- ❌ Don't use external font CDNs. Fonts are self-hosted as `.woff2` in `public/fonts/` to match the self-hosted-instance ethos.
- ❌ Don't override `<table class="table">` `<thead th>` styling — it cascades globally as tracked-uppercase mono.

**Fonts as identity:**
- `font-mono`: ability scores, AC, HP, modifiers, DCs, table headers — game-mechanical numbers read as ledger data, not decoration.
- `font-display`: headings and emphasis.
- Default to `font-sans` for prose.

**Comments:**
- ❌ Don't write comments that restate what the code says or narrate the next line.
- ✅ Write a comment only when it carries a gotcha, constraint, or non-obvious "why" (e.g., why a workaround exists, what a magic value means). One line max.
- Multi-paragraph explanations belong in commit messages or PR descriptions, not inline.

## Git workflow

- Always branch from `main`. Never commit directly to `main`.
- Branch name: `<prefix>/<short-description>` where `prefix` is `feat` (new feature), `fix` (bug fix), or `eng` (refactoring/tooling/docs).
- One clear commit per logical change. Squash WIP commits before the PR is ready.

## Further reading

- **AGENTS.md**: Architecture principles, code style, testing, common implementation patterns.
- **README.md**: Setup, commands, deployment.
- **src/lib.rs, src/main.rs**: Feature gate structure (WASM vs. server).
- **src/pages/compendium.rs**: Reference example for landing/grid pages.
- **style/main.css**: Theme definition and signature CSS classes.
