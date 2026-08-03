# D1 — the content seam + the episode spine, read-only

Program map: `docs/plans/chat-operations-console.md` (link D1).
Branch: `feat/d1-chat-content-seam` off `main`.

## 1. Context

DevPULSE Chat is a peer window the operator opens over the app. The
operator's requirement, verbatim:

> "BOTH CHAT READS AND HAS FULL ACCESS TO THE PLATFORM THE WHOLE POINT IS
> THAT CHAT BECOMES A CLEAR POINT OF OPERATIONS TO WORK ON THE CONTENT
> WHETHER ITS RAW OR BUILDING THE OFFICIAL SLIDES, AND
> PRESENTATION/EPISODE"

Today Chat cannot see content at all. Its entire view of the platform is
a `Route` enum naming where the main window is; its rail says
"Observed from Main · context not attached" and means it literally. The
chat crate tree declares no dependency on `dpl-doc`, `dpl-comp`,
`dpl-workflows`, or the index.

This pass builds the FIRST link of the chain: the seam that carries
content truth into Chat, and the first thing it carries — the episode
spine (shows → episodes → chapters → slides), READ-ONLY. Nothing in this
pass writes to disk. It exists so that D2's navigation and D3's
authoring have a proved model to stand on.

## 2. Current state

Every line number and excerpt below was read in this session against
`56e5ec7`.

### 2.1 The seam that exists — host pushes in, window pushes intents out

`crates/app/src/chat.rs:630`:

```rust
    pub fn frame(&mut self, ctx: &egui::Context, current_route: Route) {
        let mut open_requested = false;
        {
            let mut shared = lock(&self.shared);
            shared.window.observe_route(current_route);
            drain_provider_facts(&mut shared);
            while let Ok(event) = self.events.try_recv() {
                match event {
                    ShellEvent::RouteChanged { route } => shared.window.observe_route(route),
```

`crates/chat/window/src/lib.rs:731`:

```rust
    pub fn observe_route(&mut self, route: Route) {
        self.route_context.observe(route);
    }
```

Intents return through `apply_host_intent(ctx, &shared, intent)`
(`crates/app/src/chat.rs:879`), whose arms cover provider lifecycle,
sessions, turns, imagine, assist, `NavigateTo`, `ShowMain`, `Quit`,
`OperatorSlash` — and nothing content-shaped.

**`frame()` runs every frame.** Any content read placed there would be a
filesystem scan per frame. The refresh policy in §3.4 exists because of
this line.

### 2.2 `dpl-chat-core` — the promise the seam must keep

`crates/chat/core/src/lib.rs:1`:

```rust
//! Provider-neutral Chat identities and state contracts.
//!
//! This crate deliberately knows nothing about egui, provider processes,
//! filesystem discovery, or transport protocols.
```

and the domain enum at `:12`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ChatDomain {
    #[default]
    Ai,
    Social,
}

impl ChatDomain {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ai => "AI",
            Self::Social => "Social",
        }
    }
}
```

### 2.3 Per-domain state — `crates/chat/window/src/lib.rs:196`

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChatDomainStates {
    pub ai: DomainViewState,
    pub social: DomainViewState,
}

impl ChatDomainStates {
    pub const fn get(&self, domain: ChatDomain) -> &DomainViewState { ... }
    pub const fn get_mut(&mut self, domain: ChatDomain) -> &mut DomainViewState { ... }
}
```

Both accessors are `const fn` over an exhaustive match. A **unit**
variant keeps them `const`; only a data-carrying variant would not (this
is precisely the cost that got `ChatDomain::Provider` rejected in M3 —
see `docs/plans/chat-connected-provider-tabs.md` §3.1).

### 2.4 What content truth is available, and from where

All in `crates/doc/src/lib.rs` — ceiling `serde, serde_json,
dpl-coldlight`, so it is a document crate doing its own file IO:

```rust
pub struct Episode { pub v: u32, pub id: String, pub title: String,
                     pub chapters: Vec<Chapter> }          // :467
pub struct Chapter { pub id: String, pub title: String,
                     pub slides: Vec<String> }             // :476
pub struct SlideRef { pub chapter: String, pub name: String,
                      pub dir: PathBuf, pub index: usize }  // :486
pub struct EpisodeEntry { pub show: Option<String>,         // :929
                          pub id: String, pub dir: PathBuf }
pub fn all_episodes(content_root: &Path) -> Vec<EpisodeEntry>  // :993
pub fn load_episode(dir: &Path) -> Result<Episode, DocError>   // :494
pub fn episode_slides(dir: &Path, ep: &Episode) -> Vec<SlideRef> // :510
pub fn load(dir: &Path) -> Result<Slide, DocError>             // :166
```

`dpl_doc::load` — NOT `load_slide`, which does not exist — is how a
slide document is read from its folder; `crates/hubs/workbench/src/
author.rs:964` is the precedent call site.

`show: None` marks a SEED template; `Some(show)` marks operator content.
The content root resolves through `dpl_engine_capture::content_root()`
(`crates/engine-capture/src/lib.rs:114`, a cached `OnceLock`).

**A `Slide` has no title** (`crates/doc/src/lib.rs:27` — `v`, `template`,
`background`, `blocks`, `enter`, `reveals`). Its human name is its folder,
i.e. `SlideRef.name`. The deck already labels a slide
`format!("{} · {}", here.chapter, here.name)`
(`crates/deck/src/lib.rs:532`) — D1 reuses that convention rather than
inventing a second one.

### 2.5 The address grammar already in use

`crates/hubs/workbench/src/lib.rs:193`:

```rust
    pub fn open_episode(&mut self, id: &str) {
        let union = dpl_doc::all_episodes(&dpl_engine_capture::content_root());
        let hit = match id.split_once('/') {
            Some(("template", ep)) => union.into_iter().find(|e| e.show.is_none() && e.id == ep),
            Some((show, ep)) => union
                .into_iter()
                .find(|e| e.show.as_deref() == Some(show) && e.id == ep),
            None => union.into_iter().find(|e| e.id == id),
        };
```

`"<show>/<ep>"` or `"template/<ep>"`. D1 adopts this grammar verbatim so
Chat's addresses are the same strings the `--open-episode` harness seam
already accepts — which is what makes D2's navigation a one-line host
call instead of a new resolver.

### 2.6 Where the currently-open episode lives — and why D1 does not use it

`crates/hubs/workbench/src/lib.rs:105`:

```rust
    /// The episode-wall's live slide docs, loaded once per episode open.
    episode_view: Option<EpisodeView>,
```

The open episode is **hub-private state**. Hubs never depend on one
another and Chat is a surface, not a hub, so there is no legal path from
Chat to "what the Workbench has open", and no shell-level current-episode
exists to read instead. D1 therefore shows the **census** — what the
operator HAS — not "what is open". Mirroring the open episode requires a
new shell-level signal; that is D2's business, not D1's.

### 2.7 `ChatDomain` blast radius

24 exhaustive match arms in `crates/chat/window/src/lib.rs`; exactly one
site in `crates/app/src/chat.rs:277`. Grep-verified this session.

## 3. Target design

### 3.1 The seam carries plain data; the host translates

`dpl-chat-core` gains content VIEW types that are owned plain data — no
`PathBuf`, no `dpl_doc` types, no serde requirement. The HOST reads
`dpl_doc` truth and translates. This keeps `dpl-chat-core`'s doc-header
promise true, keeps `dpl-chat-window` free of any content dependency, and
makes Chat content-SOURCE-neutral exactly as it is already
provider-neutral.

```rust
/// Address of one episode in the seeds+content union. The grammar is the
/// harness grammar: "<show>/<episode>", or "template/<episode>" for a seed.
pub struct ContentEpisodeId(pub String);

pub struct ContentEpisode {
    pub id: ContentEpisodeId,
    /// None = a seed template, not operator content.
    pub show: Option<String>,
    pub title: String,
}

pub struct ContentSlide {
    pub chapter: String,
    pub name: String,
    pub template: String,
    pub index: usize,
}

pub struct ContentChapter {
    pub id: String,
    pub title: String,
    pub slides: Vec<ContentSlide>,
}

/// One episode opened for inspection.
pub struct ContentSpine {
    pub id: ContentEpisodeId,
    pub title: String,
    pub chapters: Vec<ContentChapter>,
}

/// Everything Chat knows about the operator's content.
#[derive(Default)]
pub struct ContentView {
    pub episodes: Vec<ContentEpisode>,
    pub spine: Option<ContentSpine>,
    pub error: Option<String>,
}
```

`error` is a field, not a panic path: an unreadable content root is a
truthful state Chat displays, never a crash (the truthful-activation
doctrine, applied to content).

### 3.2 Content is a third DOMAIN, not a section of the AI rail

`ChatDomain::Content` — a **unit** variant. Content is not AI's and not
Social's; burying the operator's episodes under "AI PROVIDERS" would be
the wrong home and would have to be moved in D3, which is exactly the
rework this program is sequenced to avoid.

Because the variant carries no data, `ChatDomainStates::get/get_mut`
gain a third arm and stay `const fn`, and `ChatDomainStates` gains one
`content: DomainViewState` field. The M3 rationale for rejecting
`ChatDomain::Provider(ProviderId)` does not apply — that variant's cost
was entirely in the data it carried.

- Tab strip: `AI · <connected providers> · Content · Social`. Content
  sits after the AI cluster and before Social.
- Rail (`content_sidebar`): the episode census, grouped SHOWS first then
  SEED TEMPLATES, each row selectable.
- Pane (`content_board`): the selected episode's spine — chapters, and
  their slides labelled `<chapter> · <name>` with the template beneath.
  With nothing selected: the census count and a line naming the content
  root's state.

### 3.3 The read loop — roster → select → detail

Mirrors the provider/session shape exactly:

```
 census pushed in  ──►  operator taps an episode row
                              │
                              ├─ ChatWindowIntent::SelectContentEpisode(id)
                              │       (window records the focus)
                              └─ HostIntent::LoadContentSpine(id)
                                        │
 host loads that ONE episode ◄──────────┘
        │
        └─ window.observe_content_spine(spine)  ──►  pane renders
```

Only the SELECTED episode's spine is loaded. The census is a directory
walk; loading every episode's chapters up front would scale with the
operator's whole library for no gain.

### 3.4 Refresh policy — never per frame

`frame()` runs every frame, so content is read on explicit triggers only:

0. **at construction of the `ChatWindow`** — the census is seeded before
   the window can be shown by ANY path,
1. when the chat window is opened via `OpenOrFocus`,
2. when the operator taps the rail's refresh affordance,
3. after a `LoadContentSpine` intent, for that episode alone.

Trigger 0 is load-bearing and was MISSING from this plan's first draft:
the window has more than one way to be opened — `--chat-open` opens it
without ever raising a `WindowRequest` — so binding the only read to the
open event left the census permanently empty on that path. The drill
caught it (step 7). The open event is a REFRESH, never the sole read.

The host caches the translated `ContentView` and pushes it via
`observe_content`. D3+ will add "after our own write" to this list; that
is why the trigger set is a named policy and not an inline call.

### 3.5 Scale story

Adding the Nth episode costs one census row (a `read_dir` entry, already
paid by `all_episodes`). Adding the Nth slide costs one row inside the
selected spine only. Nothing in this design walks the whole library's
slides, and nothing re-reads on a frame tick.

## 4. Changes, in execution order

Each step is verified before the next begins (§6 runs interleaved, not
only at the end).

### Step 1 — `crates/chat/core/src/lib.rs`: the view types

Add the six types from §3.1 verbatim, with `#[derive(Debug, Clone,
PartialEq)]` on each (`Eq` where no field forbids it) and
`#[derive(Default)]` on `ContentView`. Add `ContentEpisodeId` as a plain
newtype over `String` (NOT via the `static_id!` macro at `:28` — that
macro mints `&'static str` ids, and these are runtime values).

### Step 2 — `crates/chat/core/src/lib.rs`: the domain variant

```rust
pub enum ChatDomain {
    #[default]
    Ai,
    Content,
    Social,
}
```

and `label()` gains `Self::Content => "Content"`. Add a unit test
`content_is_a_domain_with_its_own_label` beside the existing
`ai_is_the_default_chat_domain` (`:556`).

### Step 3 — `crates/chat/window/src/lib.rs`: `ChatDomainStates`

Add `pub content: DomainViewState` and the third arm to both `const fn`
accessors. Compile; the compiler now names every exhaustive match that
must be handled in Step 4 — work that list, do not hunt by hand.

### Step 4 — `crates/chat/window/src/lib.rs`: the domain arms

Add a `ChatDomain::Content` arm to each site the compiler names (24
arms). Rail → `content_sidebar`; conversation → `content_board`;
placeholder/copy sites get content-true strings
(e.g. `:3198`'s picker prompt → "Choose an episode"). No arm may be
`unreachable!()` or `_ =>`.

### Step 5 — `crates/chat/window/src/lib.rs`: state, intents, observers

- Field `content: ContentView` (host-pushed) and
  `content_focus: Option<ContentEpisodeId>` (window-owned).
- `pub fn observe_content(&mut self, view: ContentView)` and
  `pub fn observe_content_spine(&mut self, spine: ContentSpine)`,
  beside `observe_route` at `:731`.
- `ChatWindowIntent::SelectContentEpisode(ContentEpisodeId)` and
  `ChatWindowIntent::RefreshContent`.
- `HostIntent::LoadContentSpine(ContentEpisodeId)` and
  `HostIntent::RefreshContent`.
- `apply` arms: `SelectContentEpisode` sets `content_focus`, clears a
  stale `spine`, and returns `Ok(Some(HostIntent::LoadContentSpine(id)))`;
  `RefreshContent` returns `Ok(Some(HostIntent::RefreshContent))`.

### Step 6 — `crates/chat/window/src/lib.rs`: `content_sidebar`

Census rows grouped `SHOWS` then `SEED TEMPLATES`, using the existing
`micro_heading` + `sidebar_row` idiom (as `session_groups` does at
`:1625`). Pilot ids `chat.sidebar.content/<address>` under a new
`pub const CONTENT_ID: PilotId = PilotId::new("chat.sidebar.content")`.
Each row: `Role::Tab`, `metadata.selected` = focus match,
`metadata.value` = the show or "seed template". Plus a
`⟳  Refresh` destination row emitting `RefreshContent`.

### Step 7 — `crates/chat/window/src/lib.rs`: `content_board`

Selected spine: chapter headings, then one row per slide labelled
`format!("{} · {}", chapter, name)` (§2.4's convention) with
`metadata.value` = the template. Nothing selected: the census count and,
if `content.error` is `Some`, that error in `Tone::Coral`. Pilot ids
under `chat.content.board`.

### Step 8 — `crates/app/src/chat.rs`: the host translator

A private `fn read_content_view() -> ContentView` calling
`dpl_doc::all_episodes(&dpl_engine_capture::content_root())`, mapping
each `EpisodeEntry` to a `ContentEpisode` whose address follows §2.5's
grammar and whose `title` comes from `load_episode(&entry.dir)` when it
parses and falls back to the entry id when it does not. A
`fn read_content_spine(id) -> Option<ContentSpine>` resolving the address
the same way `open_episode` does, then `load_episode` +
`episode_slides`, reading each slide's `template` via
`dpl_doc::load(&slide_ref.dir)` and falling back to `"unknown"` when a
slide folder does not parse (a malformed slide is a displayable fact,
never a failed episode). Both are host-side; neither is called from
`frame()`'s hot path.

### Step 9 — `crates/app/src/chat.rs`: wiring

- **No `Cargo.toml` change.** Verified this session: `crates/app/
  Cargo.toml:35` already declares `dpl-doc.workspace = true` and `:36`
  `dpl-engine-capture.workspace = true`. If a step appears to need a new
  app dependency, re-ground before adding one.
- `apply_host_intent` gains `LoadContentSpine` and `RefreshContent` arms
  calling the Step 8 readers and pushing through `observe_content*`.
- The `OpenOrFocus` path (`crates/app/src/chat.rs:641`) refreshes the
  census once on open — trigger 1 of §3.4.

### Step 10 — unit pins (`crates/chat/window/src/lib.rs` tests)

- `content_is_its_own_domain_and_keeps_state` — selecting `Content` then
  `Ai` preserves each domain's `DomainViewState` independently.
- `selecting_an_episode_asks_the_host_for_its_spine` —
  `SelectContentEpisode` returns `HostIntent::LoadContentSpine` with the
  same address.
- `an_observed_spine_replaces_the_previous_one` — two
  `observe_content_spine` calls leave exactly the second.
- `a_content_error_is_state_not_a_panic` — a `ContentView` with `error`
  set renders and reports, never unwraps.

### Step 11 — `tests/eguipilotai/chat-content-spine.toml`

Fixture: a NEW `chat-content` fixture in `.eguipilotai.toml` pointing
`CONTENT_ROOT` at a scratch tree under `target/` (the `wiredfront-workflows`
fixture at `.eguipilotai.toml:77` is the precedent — drill roots never
touch `/content`). Steps: viewport open → wait → focus → dismiss the
authored approval → click `chat.primary/chat.mode.content` → expect the
census row visible → click it → expect a spine slide row visible →
`count = 0` on a row from a DIFFERENT episode (proving scoping) →
close-viewport.

## 5. Boundaries

🚫 **Never touch**

- **`/content`** — operator data. This pass is READ-ONLY by design; the
  EguiPilot fixture points `CONTENT_ROOT` at a scratch tree so even the
  test never opens the real one. No code path in this pass writes,
  renames, or deletes anything under any content root.
- **Seed episodes** (`seeds/episodes/`) — read, never written.
- `crates/hubs/workbench` — the open-episode state stays hub-private
  (§2.6). Chat does not reach into a hub.
- The activation/provider paths — untouched, as in M1/M2/M3.
- `dpl-chat-window`'s dependency list — it must NOT gain `dpl-doc`
  (§3.1). If a step seems to need it, the design is wrong, not the law.

⚠️ **Ask first**

- Any change to `dpl-doc`. This pass consumes it and does not amend it.
- Adding a dep ceiling row or a new crate — neither is needed here; if
  one becomes necessary, stop and amend `docs/design/architecture.md`
  plus the xtask audit table in the same commit.

**Out of scope (later links, by design)**

- Any mutation of episodes or slides (D3).
- Raw media (D4) — the two-libraries law keeps it a separate surface.
- Workflow stages (D5), AI drive (D6).
- Mirroring "what the Workbench has open" — needs a shell signal (D2).

## 6. Verification

Run interleaved with §4, in this order; each is pass/fail.

1. After Step 2: `cargo test -p dpl-chat-core` → passes incl. the new
   label test.
2. After Step 4: `cargo check -p dpl-chat-window` → exit 0 with no
   non-exhaustive-match errors remaining.
3. After Step 10: `cargo test -p dpl-chat-window` → all prior tests plus
   the four new pins pass; zero failures.
4. `cargo clippy -p dpl-chat-core -p dpl-chat-window -p devpulselabs
   --all-targets -- -D warnings` → exit 0, no `#[allow]` added.
5. `git diff --stat origin/main...HEAD` — `crates/` touched is EXACTLY
   `chat/core`, `chat/window`, `app`. Any other crate means a boundary
   broke.
6. `grep -n "dpl-doc" crates/chat/*/Cargo.toml crates/chat/*/*/Cargo.toml`
   → **no output**. This is the §3.1 law, mechanically checked.
7. `cargo green` on the server1 lane → `green ✔` (includes the
   `structure` phase, which enforces the layer stack).
8. EguiPilot, SEQUENTIAL, discarding the first run after any rebuild
   (the warm-up handshake race — see
   `docs/journal/2026-07-29-connected-provider-tabs.md`):
   `chat-content-spine` (new) → pass; then `chat-provider-tab`,
   `chat-provider-rows`, `chat-provider-catalog`, `chat-assist-toggle`
   → all pass unregressed.
9. End-to-end proof on the REAL exe: rebuild
   `target\release\devpulselabs.exe` **with**
   `--features eguipilotai-devtools` first (the `chat-native-proof` shim
   spawns `--chat-proof`, which is gated), then capture at a real window
   size. The shot must show the Content tab and a real episode spine.
   Rebuild the exe PLAIN afterwards — that is the operator's artifact.

## 7. Commit plan

Branch `feat/d1-chat-content-seam` off `main`. One commit:

```
feat(chat): the content seam — Chat reads the episode spine (D1)
```

`git add` — `crates/chat/core/src/lib.rs`,
`crates/chat/window/src/lib.rs`, `crates/app/src/chat.rs`,
`.eguipilotai.toml`,
`tests/eguipilotai/chat-content-spine.toml`,
`docs/plans/chat-operations-console.md`,
`docs/plans/chat-content-seam.md`,
`docs/journal/<date>-chat-content-seam.md`,
`docs/proof/chat-content-spine-native.png`, `docs/STATE.md`. Nothing
else — the untracked files in the tree are parallel sessions' work.

`gh pr list` before authoring (the chat window crate is a collision
magnet). PR via `gh pr create`; merge ONLY on double-green.

## 8. Evidence

Filled during execution.
