# Test fixtures

Original, made-up compendium content used by the end-to-end tests and CI.
Unlike `fixtures/` (gitignored — users import their own rulebook content),
everything here is invented for this repo and safe to commit.

The files mirror the shapes the importer expects, one per content type, so the
seed script can load them with `FIXTURES_DIR=test-fixtures`:

```bash
FIXTURES_DIR=test-fixtures DATABASE_URL=sqlite://e2e.db SEED_RESET=1 cargo run --features ssr --bin seed
DATABASE_URL=sqlite://e2e.db cargo leptos end-to-end
```

The Playwright specs assert against these exact records (names, sources,
counts), so extending a file usually means updating `end2end/tests/` to match.

`items.json` (`item`/`itemGroup` arrays) and `items-base.json`
(`baseitem`/`itemProperty`/`itemType` arrays) together cover the items
compendium: a mundane weapon, a magic item with rarity/attunement/properties,
a `_copy`+`_mod.insertArr`-derived variant of that item, and an item group.
