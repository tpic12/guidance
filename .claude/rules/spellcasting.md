---
name: spellcasting
paths: ["src/importer/parse_class_spells.rs", "src/importer/transform_class_spells.rs", "src/models/class.rs", "src/db.rs"]
---

## Spellcasting Reference

**Class features:**
- `caster_progression`: normalized at import time from fraction codes (`"1/2"` → `"half"`, `"1/3"` → `"third"`).
- `spells_known_progression` / `cantrips_known_progression`: fixed per-level tables; their presence marks a *known*-caster (Sorcerer) vs. a *prepared*-caster (Wizard/Paladin).
- Prepared-caster spell count is derived via `spells_known_count`'s hardcoded formula (full/half depending on class).

**Class-spell link:** not stored on the spell record; comes from a separate lookup file mirrored into `fixtures/`.

**Import pipeline:**
1. `src/importer/parse_class_spells.rs` / `transform_class_spells.rs` — parse and flatten the lookup into `ClassSpellLink`s.
2. `src/bin/seed.rs:seed_class_spells` — resolves those against already-inserted `classes`/`spells` rows by name into the `class_spells` join table.
3. **Must run after** both `seed_spells` and `seed_classes`; silently skips (tolerates dangling) any link whose class or spell isn't present.

**Schema rule:** Store inputs, derive outputs. Foreign key over duplication — if a value could change via errata/re-import, reference it, don't copy it.
