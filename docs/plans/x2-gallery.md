# X2 — the Gallery + inbox review (dump) over the index

Parent: roadmap Program X (census #2). Ledger #107. After X1.
Worktree: `C:\localdev\dpl-main` on main.

## 1. Context

X1 made captures queryable; X2 makes them VISIBLE and reviewable — the
operator's literal pain ("no way of seeing any of my previous test").
The wiredFRONT GALLERY subroute (placeholder today) becomes the content
library wall: every artifact thumbnailed with honest fallbacks, filter
chips, rescan, and DUMP (the inbox-only delete with companions).

## 2. Current state (ground-truthed this session)

- X1 landed: `dpl-index` (turso, operator-ratified) with Record/upsert/
  soft_clear/reindex/CLI; all 5 pixvix save paths write through;
  `content_root()/inbox` holds pix/vix/guide artifacts + sidecars.
- `crates/hubs/wiredfront` (read in full): stateless `WiredfrontHub`,
  GALLERY subroute renders the shared placeholder.
- `main.rs` installs `egui_extras::install_image_loaders` incl. the
  `file://` loader — `egui::Image::new("file://<abs>")` renders thumbs.
- Census filing laws: dump = delete inbox artifact + companions (guide
  folders recursively) + SOFT clear in the index; the inbox file JAIL
  (never a path outside inbox, no traversal). Keep/associate move files
  INTO episode content — that needs the operator-content shows-tree
  doctrine the rewrite hasn't ratified yet.

## 3. Target design

**Scope split (roadmap amended this pass):** X2 = Gallery + DUMP +
rescan. KEEP-into-episode moves to X3 with associate/import — all three
are the same "filing into content" doctrine, and the rewrite's operator
content tree (old repo: `content/shows/<show>/<ep>/generated/`) is a
design the operator ratifies once, not something to improvise here.
(Episode folders' doctor-enforced contract currently has no artifact
home; X3 proposes one.)

**`dpl-index` grows `dump(root, rel_path)`** — engine-side, headless-
testable: the JAIL (rel must start `inbox/`, no `..` components, the
canonicalized target must live under `root/inbox`), then delete the
artifact (a guide folder recursively), its sidecar, and a video's
poster companion; finally `soft_clear` (history survives). Never
touches anything outside the inbox — by construction.

**The Gallery** (wiredFRONT GALLERY subroute; hub becomes stateful):
- Loads `Index::open_default().recent(500, None)` on first visit;
  RESCAN chip runs `reindex(content_root())` + reload and reports the
  honest numbers; reload chip after external edits.
- Filter chips: all · image · video · guide (PilotIds
  `gallery.filter.<kind>`, `gallery.rescan`).
- Card grid (adaptive columns): thumbnail — image → the file itself
  via `file://`; video → its poster if present; guide → its
  `step-001.webp` when present; anything else → the kind label on a
  RAISED plate (honest fallback, never a broken image). Name, kind +
  tags line, captured_at.
- DUMP per card, two-tap armed ("Dump?" then confirm within the same
  card; disarms on any other interaction) — destructive actions never
  fire on one stray tap (touch law). Runs `dump`, reloads, reports.
- Empty state points at PixVix.

## 4. Changes, in execution order

1. `crates/index/src/lib.rs` — `pub fn dump(&self, root, rel) ->
   Result<(), String>` + jail helper; tests: dumps a pix (file + sidecar
   gone, row soft-cleared, OTHER rows intact), dumps a guide folder
   recursively, dumps a video + poster companion, REFUSES
   `shows/...`, `inbox/../x`, absolute paths, and a rel that resolves
   outside the inbox.
2. `crates/hubs/wiredfront` — Cargo.toml gains dpl-index +
   dpl-engine-capture + egui_extras(workspace); lib.rs: stateful hub +
   `GalleryView { rows, filter, status, armed: Option<String> }` +
   `gallery_surface`; placeholder stays for the other subroutes.
3. `cargo green`; exe proof `--route wiredfront/gallery` (the real
   inbox already holds the X1 smoke capture) →
   `docs/proof/x2-gallery.png`; commit; push.

## 5. Boundaries

🚫 No keep/associate/import (X3, with the content-tree doctrine).
🚫 No editing, no ViziWiz, no sessions. 🚫 Dump never reaches outside
`root/inbox` (jail-tested). ⚠️ Gallery reads via `open_default` per
action (cheap local open; the X1 pattern) — no long-lived handle in UI
state.

## 6. Verification

Green incl. the new dump/jail suite. Exe: the gallery route shows the
real captured pix thumbnail (not a label) + rescan reports honest
numbers. Dump drill (headless test covers the destructive path; the UI
arm/confirm is kittest-smoked).

## 7. Commit plan

`feat(gallery): X2 — the Gallery wall + inbox dump over the index` on
main; push backgrounded.
