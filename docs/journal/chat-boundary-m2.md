# BND1 · M2 journal — the command strip (2026-08-08)

**Done this milestone:**
- `dpl_chat_core::matching_commands(draft)` — the typeahead's ONE
  source, derived from the census (no second filter table): `/r` →
  route + routines in registry order; `/` → all six; unknown prefix or
  past-the-slug drafts offer nothing. Pinned
  (`the_typeahead_offers_only_census_commands`).
- The strip itself (`command_strip`, window): an `Area` +
  `Order::Foreground` panel anchored above the composer (the
  provider-hover mount shape — no popup precedent existed anywhere),
  cyan-stroked, one row per match (`/slug — label`, blurb + arg asks
  in the tip), click-to-fill (`UpdateAiDraft` with a trailing space
  when the command takes args) with focus returned to the composer.
  Pilot nodes: `chat.composer.command-strip` region (value = match
  count) + per-slug children (value = slug).
- `ChatProofMode::Commands` — BOTH duplicated proof-state lists
  amended together (chat.rs enum+parse+arrangement,
  chat_native_proof usage+validator): arranges a `/ro` draft so the
  strip stands open.
- Proof PNG (`docs/proof/bnd1-command-strip.png`, READ): the strip
  offering exactly /route and /routines over the `/ro` draft, and the
  route badge reading **19 routes** — the FULL platform census where 6
  hub-defaults used to sit. Committed as the baseline.

**Proof:** 17 core + 45 window + 37 app tests green; fmt clean.

**Deviations:** arrow-key strip navigation (Up/Down/Tab selection)
DEFERRED to a later slice — the key-retain pass beside
`take_composer_submit` is designed (plan D9) but click-to-fill ships
first; logged here so it never silently reads as done.

**Remaining (POAM):** M3 Clyffy house session · M4 close-out.
