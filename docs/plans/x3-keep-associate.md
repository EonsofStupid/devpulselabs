# X3 — the content tree, keep, and associate

Parent: roadmap Program X. Ledger #107. After X1/X2.
Worktree: `C:\localdev\dpl-main` on main.

## 1. Context — the RATIFIED tree (operator, 2026-07-23)

```
content/<show-name>/<episode-name>/     the operator's REAL shows
  episode.json · theme.json · <slide>/   the SAME doctor-enforced
  assets/                               contract as seeds, plus the
                                        episode-level assets home
```

- The show name IS the stream category. The operator's shows
  (2026-07-23): **DevPULSE News** (`devpulse-news`), **Eons of
  Stupidity** (`eons-of-stupidity`), **EffingAI** (`effingai`) — the
  clone flow offers these three as suggestions when content is empty
  (kebab via the metadata law); any new category is just a folder name.
- Seeds stay pristine templates; a new show COPIES a seed into content.
- **EPISODES ARE NOT GUIDES** (operator's emphasis): the Steps/guide
  system files through the inbox and stays separate. A guide's frames
  only enter an episode via an EXPLICIT associate (or T4 later) —
  never by filing, never by scan.
- `/content` is gitignored operator data — nothing here is committed.

## 2. Current state (all ground-truthed in this session's reads)

- `dpl_doc`: `seeds_root()/list_seeds()/episodes_root()/list_episodes()`
  find `seeds/*` via `find_upward`; the folder contract knows
  `EPISODE_RESERVED = [episode.json, theme.json]` (no assets yet).
  Present + Workbench pickers list `list_episodes()` only.
- `dpl_index` (X1/X2): Record/upsert/rekey/soft_clear/dump + the inbox
  jail; sidecars; `cap::content_root()` = `./content`.
- Old filing laws (census): keep = file per metadata, episode must
  EXIST, companions follow, rekey; associate = COPY never move.

## 3. Target design

**`dpl_doc` grows the content tier (pure, root passed in):**
- `EPISODE_RESERVED` += `"assets"` (the contract grows by one line —
  its design working as intended).
- `list_shows(content_root) -> Vec<(String, PathBuf)>` — subfolders of
  the content root holding at least one episode folder (a folder with
  `episode.json`); skips `inbox/` and `artifacts.db*` by name.
- `list_content_episodes(content_root) -> Vec<(String /*show*/,
  String /*episode*/, PathBuf)>`.
- `all_episodes(content_root)` — the UNION every picker uses: seeds
  (labelled as templates) + content shows.
- `clone_episode(seed_dir, content_root, show, episode) ->
  Result<PathBuf, String>` — the "new show from a seed" engine: copies
  episode.json + theme.json + slide folders (copy, never move; refuses
  an existing destination), creates `assets/`.

**`dpl_index` grows `keep`:**
- `keep(root, rel_path, show, episode) -> Result<String /*new rel*/>`
  — inbox-jailed source (the X2 jail, reused); the destination episode
  folder MUST already exist (the old law — keep never creates shows);
  moves the artifact + companions (sidecar; video poster; a guide
  folder moves whole) into `content/<show>/<episode>/assets/` with
  never-clobber naming; rewrites the sidecar's rel_path/location;
  `rekey`s the row (`location='content'`, show_id=show,
  episode_folder=episode).
- `associate(root, rel_path, show, episode)` — same destination
  resolution, but COPY (the library master stays; a NEW row for the
  copy).

**Gallery (X2 surface) grows Keep:** each card gets "Keep" next to
"Dump"; tapping opens a small in-place target list (the union of
content episodes — seeds are NOT keep targets, they're templates);
tapping a target runs `keep` and reloads with the honest status.
PilotIds `gallery.keep`, `gallery.keep.target.<show>/<ep>`.

**Pickers union:** Present's show picker and Workbench's episodes wall
+ editor picker list `all_episodes` — content shows first (they're the
point), seeds labelled "template". Workbench episodes wall gains a
"copy to my shows" chip on seed cards → a name prompt (show category +
episode name, kebab-forced via the existing metadata law) →
`clone_episode`.

Scale story: a new stream category is a folder name; a new show is one
clone; every surface reads the same union; the doctor enforces the
same contract in both roots.

## 4. Changes, in execution order

1. `crates/doc` — contract + list_shows/list_content_episodes/
   all_episodes/clone_episode + tests (clone copies + refuses existing;
   contract accepts assets/; unions order content-first; a guide folder
   in content is NOT an episode).
2. `crates/index` — keep + associate + tests (episode-must-exist
   refused; companions travel; never-clobber; rekey row + sidecar
   rewritten; associate leaves the master; guide folder keeps whole;
   the inbox jail still binds the source).
3. `crates/hubs/wiredfront` — Gallery Keep flow (target list, status).
4. `crates/hubs/present` + `crates/hubs/workbench` — union pickers;
   "copy to my shows" chip + name prompt on the seeds wall.
5. `cargo green`; drill: seed a temp content tree, CLI/tests prove
   keep+associate; exe proofs (gallery keep list, present union picker);
   commit; push.

## 5. Boundaries

🚫 Guides never auto-enter episodes (associate/T4 only — the operator's
line). 🚫 No T4 guide-slide yet. 🚫 No delete of content artifacts
(dump stays inbox-only). 🚫 Seeds untouched at runtime (clone reads
them, never writes). ⚠️ The kebab/metadata law from engine-steps names
new shows/episodes.

## 6. Verification

Green with the new suites; the seed episode still validates (assets/
reserved); a scripted keep drill moves a pix into a cloned show and
the Gallery lists it as content; the union picker shows the cloned
show above the seed templates on the real exe.

## 7. Commit plan

`feat(content): X3 — the ratified content tree, keep, associate` on
main; push backgrounded.

## 8. Executed — deviations recorded (corrections law)

- The headless drill needed engine paths, so two harness surfaces grew
  beyond §4's list: `dpl-index-cli` gained `keep`/`associate`
  subcommands, and the app gained `--clone-seed <seed-id> <show> <ep>`
  (runs `dpl_doc::clone_episode` against the real content root and
  exits — the same do-and-exit shape as `--present`).
- The inbox jail's messages were de-dump-ified ("refused…", "not
  found…") since keep/associate reuse it verbatim.
- `Index::rekey` alone leaves poster_rel_path stale, so keep re-keys
  THEN upserts the corrected record (same new rel_path → full refresh).
- Kebab checking lives in `dpl_doc` as a restated predicate (the
  `find_upward` precedent: the document layer doesn't depend on the
  capture engine for a character check).
- Proofs: docs/proof/x3-present-union.png (devpulse-news/how-ai-works
  above the template), x3-workbench-union.png ("copy to my shows"
  chip), x3-gallery-keep.png (Keep beside Dump); CLI drill moved a pix
  + associated a guide into a temp tree with honest refusals.
  `content/devpulse-news/how-ai-works` is the first REAL show on disk,
  cloned through the engine.
