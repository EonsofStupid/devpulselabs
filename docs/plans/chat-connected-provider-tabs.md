# Connected-provider tabs (POA&M M3)

## 1. Context

DevPULSE Chat is a peer window (Steam-overlay style) whose title bar
carries a tab strip: **AI** and **Social**. The operator's regression
report named a third behaviour that has never existed in code:

> "then it triggers inside the chat the abiltiy to log in to
> subscription, once connected it has its own tab and then sessions had"

M1 restored the always-visible provider roster in the AI rail and M2
proved the activation/login flow still fires from it. This pass builds
the third clause: **once a provider is connected it earns its own tab**,
and that tab scopes the rail to that provider's sessions.

POA&M row (docs/plans/poam-2026-07-29.md, M3):

> Connected-provider tabs: Ready providers earn a tab beside AI/Social;
> that provider's sessions beneath. Exit: tab appears on Ready,
> disappears on sign-out; sessions listed under the tab; AI tab remains
> the roster/fallback.

## 2. Current state

All line numbers ground-truthed in-session against `e641500`
(feat/m1-provider-easy-select, rebased on `fae7c34`).

### 2.1 The domain enum — `crates/chat/core/src/lib.rs:12`

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

`ProviderId` is `pub struct ProviderId(pub &'static str)` — `Copy`,
`'static` (`core/src/lib.rs:41`, via the `static_id!` macro).

### 2.2 Per-domain view state — `crates/chat/window/src/lib.rs:193`

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

Both accessors are `const fn` over an exhaustive match — a third
data-carrying variant would force them off `const` and into a map.

### 2.3 The tab strip — `crates/chat/window/src/lib.rs:1195-1212`

```rust
if mode_button(ui, AI_MODE_ID, "AI", self.domain == ChatDomain::Ai).clicked() {
    intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Ai));
}
let social_label = if self.domain_states.social.unread > 0 {
    format!("Social  {}", self.domain_states.social.unread)
} else {
    "Social".to_owned()
};
if mode_button(ui, SOCIAL_MODE_ID, &social_label, self.domain == ChatDomain::Social).clicked() {
    intents.push(ChatWindowIntent::SelectDomain(ChatDomain::Social));
}
```

Ids: `AI_MODE_ID = "chat.mode.ai"`, `SOCIAL_MODE_ID = "chat.mode.social"`
(`lib.rs:41-42`).

`mode_button` (`lib.rs:3452`) allocates a fixed
`TAB_SIZE = vec2(112.0, 35.0)`, paints fill/stroke/top-rule by
`selected`, and picks its accent dot by identity:

```rust
let accent = if id == AI_MODE_ID {
    Tone::Cyan.color()
} else {
    Tone::Violet.color()
};
```

so every non-AI tab is currently violet. It emits
`NodeMetadata::new(id, Role::Tab, label.to_owned())` (`lib.rs:3511`).

The strip shares its `ui.horizontal` row with the brand mark (166px),
a search field capped at 260px, and right-to-left window buttons; the
row already degrades at `available_width() < 820.0` via `compact`
(`lib.rs:1167`).

### 2.4 The AI rail — `crates/chat/window/src/lib.rs:1298`

```rust
fn ai_sidebar(&self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
    let providers = self.runtime.provider_views();
    ...
            self.provider_roster(ui, &providers, intents);
            ui.add_space(9.0);
            self.session_groups(ui, &providers, intents);
    ...
```

`session_groups` (`lib.rs:1625`) takes `providers: &[ProviderView]` and
is already provider-agnostic: it flat-maps sessions across whatever
slice it is handed, into PINNED / ACTIVE / RECENT groups, with pilot
ids `chat.sidebar.sessions/<provider>/<session>` (`lib.rs:1703-1705`).
Its empty-state branch (`lib.rs:1631`) fires when the handed slice has
no sessions at all.

**This is the seam**: scoping the rail to one provider is handing
`session_groups` a one-element slice. No change to its body.

### 2.5 Activation truth — `crates/chat/core/src/lib.rs:310`

```rust
pub const fn is_activated(&self) -> bool {
    matches!(self, Self::Ready | Self::Experimental { .. })
}
```

`ProviderRegistration` (`crates/chat/runtime/src/lib.rs:15`) carries
`activation: ActivationState` and `sessions: Vec<SessionSummary>`;
`ProviderView` (`runtime/src/lib.rs:68`) wraps it with `active_session`
and `draft`.

### 2.6 Consumers of `ChatDomain` outside chat core/window

Exactly one, grep-verified:

- `crates/app/src/chat.rs:9` (import), `:277` (`ChatDomain::Social`).

## 3. Target design

### 3.1 The decision: tab focus is window state, not a domain variant

**Rejected — `ChatDomain::Provider(ProviderId)`.** It reads elegant but
costs: `ChatDomainStates::get/get_mut` lose `const` and become a map
allocation per provider; `DomainViewState` (scroll/unread/overlay) would
be minted per provider for no behaviour we ship in this pass; every
exhaustive match on `ChatDomain` across window + app grows a third arm;
and `ChatDomain::label()` cannot return the provider's display label
from a `&'static str` id without a lookup. Large blast radius, no
behavioural gain.

**Ratified — `provider_tab: Option<ProviderId>` on the window.** The
domain stays `Ai` while a provider tab is focused, so every AI-domain
code path (conversation, composer, drafts, overlays) keeps working
untouched. The tab strip renders AI · <connected providers> · Social;
the rail swaps roster-and-all-sessions for that-provider's-sessions.

### 3.2 Which providers earn a tab

`registration.activation.is_activated()` — `Ready` or `Experimental`.
Nothing else. Order follows `runtime.provider_views()` (registration
order) so the strip does not reshuffle under the operator's finger.

### 3.3 Selection is derived, never stored twice

| strip state | condition |
|---|---|
| AI selected | `domain == Ai && provider_tab.is_none()` |
| provider tab selected | `domain == Ai && provider_tab == Some(id)` |
| Social selected | `domain == Social` |

Clearing rules (all in `apply`, one place):

- `SelectDomain(_)` → `provider_tab = None` (AI is the fallback; Social
  has nothing to do with providers).
- `SelectProviderTab(id)` → `domain = Ai`, `provider_tab = Some(id)`,
  and `SelectProvider(id)` semantics so the composer targets it.
- Any render where `provider_tab` names a provider that is no longer
  activated (or no longer registered) → the tab is not drawn and the
  focus falls back to AI. Sign-out therefore removes the tab **and**
  returns the roster with no extra intent.

Rail select (M1's `provider_row`) deliberately does **not** set
`provider_tab`: choosing a provider from the roster keeps you on the
roster. The tab is a destination you enter, not a side effect of
selection. This is what "AI tab remains the roster/fallback" means.

### 3.4 Scale story

Adding the Nth provider costs one `mode_button` when it is connected.
The strip already computes `compact` from available width; provider
tabs use the same `compact` signal to drop to `descriptor.short_label`,
and the strip clips rather than wraps (the row is one `ui.horizontal`).
Beyond what fits, the roster and `＋ More providers…` remain the
complete census — the strip is a shortcut, never the only door.

## 4. Changes, in execution order

### Step 1 — `crates/chat/window/src/lib.rs`: the intent

Add to `ChatWindowIntent` (after `SelectDomain`, `lib.rs:217`):

```rust
    SelectProviderTab(ProviderId),
```

### Step 2 — `crates/chat/window/src/lib.rs`: the state field

Add `provider_tab: Option<ProviderId>` beside `domain` on the window
struct (the field initialised at `lib.rs:565`, `domain: ChatDomain::Ai`),
defaulting to `None`.

### Step 3 — `crates/chat/window/src/lib.rs`: `apply` (line 751)

`apply` returns `Result<Option<HostIntent>, ChatWindowError>`. Before:

```rust
            ChatWindowIntent::SelectDomain(domain) => {
                self.domain = domain;
                Ok(None)
            }
            ChatWindowIntent::SelectProvider(provider) => {
                self.runtime.select_provider(provider)?;
                Ok(Some(HostIntent::ProviderSelected(provider)))
            }
```

After:

```rust
            ChatWindowIntent::SelectDomain(domain) => {
                self.domain = domain;
                self.provider_tab = None;
                Ok(None)
            }
            ChatWindowIntent::SelectProvider(provider) => {
                self.runtime.select_provider(provider)?;
                Ok(Some(HostIntent::ProviderSelected(provider)))
            }
            ChatWindowIntent::SelectProviderTab(provider) => {
                self.runtime.select_provider(provider)?;
                self.domain = ChatDomain::Ai;
                self.provider_tab = Some(provider);
                Ok(Some(HostIntent::ProviderSelected(provider)))
            }
```

The runtime call precedes the state writes so a `RegistryError` leaves
the tab focus untouched (the `?` returns before either assignment).

### Step 4 — `crates/chat/window/src/lib.rs`: `connected_tabs` helper

```rust
fn connected_tabs(&self) -> Vec<(ProviderId, &'static str, &'static str)> {
    self.runtime
        .provider_views()
        .iter()
        .filter(|provider| provider.registration.activation.is_activated())
        .map(|provider| {
            let d = provider.registration.manifest.descriptor;
            (d.id, d.label, d.short_label)
        })
        .collect()
}
```

### Step 5 — `crates/chat/window/src/lib.rs`: render the tabs (1195-1212)

Between the AI button and the Social button, iterate `connected_tabs()`:
each emits `mode_button(ui, PROVIDER_TAB_ID.child(&PilotKey::new(id.0)),
label_or_short, self.domain == ChatDomain::Ai && self.provider_tab == Some(id))`
and pushes `SelectProviderTab(id)` on click. New const beside
`AI_MODE_ID`:

```rust
pub const PROVIDER_TAB_ID: PilotId = PilotId::new("chat.mode.provider");
```

AI's `selected` argument becomes
`self.domain == ChatDomain::Ai && self.provider_tab.is_none()`.

### Step 6 — `crates/chat/window/src/lib.rs`: `mode_button` accent

The `if id == AI_MODE_ID { Cyan } else { Violet }` branch (`lib.rs:3490`)
gains a third case so provider tabs are not mistaken for Social. Provider
tabs take `Tone::Cyan` (they are AI-domain) — Social keeps Violet:

```rust
let accent = if id == SOCIAL_MODE_ID {
    Tone::Violet.color()
} else {
    Tone::Cyan.color()
};
```

### Step 7 — `crates/chat/window/src/lib.rs`: scope the rail (`ai_sidebar`)

After `let providers = self.runtime.provider_views();`, derive the
focused slice; when `provider_tab` names a still-activated provider,
skip `provider_roster` and hand `session_groups` the one-element slice
(plus a heading naming the provider and a "‹ All providers" row that
pushes `SelectDomain(ChatDomain::Ai)`). Otherwise the existing
roster-plus-all-sessions path runs unchanged.

### Step 8 — unit pins (`crates/chat/window/src/lib.rs` tests)

- `only_activated_providers_earn_a_tab` — a registry with one Ready and
  one SignedOut provider yields exactly one entry from `connected_tabs`.
- `selecting_a_domain_clears_the_provider_tab` — `SelectProviderTab`
  then `SelectDomain(Ai)` leaves `provider_tab == None`.
- `a_provider_tab_selects_its_provider` — `SelectProviderTab(grok)`
  leaves `selected_provider_id() == Some(grok)`.

### Step 9 — `tests/eguipilotai/chat-provider-tab.toml`

Fixture `chat-authored`, `record = true`. Steps: expect viewport open →
wait 750ms → focus-viewport → dismiss the interaction overlay (the
`chat.primary/chat.overlay/interaction-1003/reject` click the
`chat-provider-rows` scenario uses) → expect `visible`
`chat.primary/chat.mode.provider/<ready-id>` → expect `hidden`
`chat.primary/chat.mode.provider/<signed-out-id>` → click the ready tab
→ expect `visible` a session node under
`chat.primary/chat.sidebar.sessions/<ready-id>/...` → click
`chat.primary/chat.mode.ai` → expect `visible`
`chat.primary/chat.sidebar.providers/<signed-out-id>` (the roster is
back) → close-viewport.

The concrete provider ids come from the `chat-authored` fixture at
execution time — read it, do not assume.

## 5. Boundaries

🚫 **Never touch**

- `/content` — operator data, gitignored (house law).
- `crates/chat/core` `ActivationState` / `ChatDomain` semantics —
  §3.1 rejected the enum change for this pass.
- The activation/login paths (`SelectProvider` runtime call chain,
  Grok/Codex contracts) — M2 proved them; this pass must again show
  zero diff lines there.
- `presence_rail` (`lib.rs:1454`) — the compact composition is its own
  experience question.

⚠️ **Ask first**

- Any change to `mode_button`'s `TAB_SIZE` or the title-row layout
  budget: it is shared with the brand mark and search field, and the
  operator has redlines pending on chat chrome.

**Out of scope**

- Per-provider unread counts on tabs (needs a `DomainViewState` per
  provider — the §3.1 cost we deliberately did not pay).
- Drag-reordering or pinning tabs.
- Social-side equivalents.

## 6. Verification

In dependency order; every criterion pass/fail.

1. `cargo test -p dpl-chat-window` — all existing tests plus the three
   new pins pass, zero failures.
2. `cargo clippy -p dpl-chat-window --all-targets -- -D warnings` —
   exit 0, no `#[allow]` added anywhere.
3. `git diff --stat origin/main...HEAD` — the only `crates/` file
   touched is `crates/chat/window/src/lib.rs` (proves the M2
   activation-continuity boundary held).
4. `cargo green` on the server1 lane → `green ✔`.
5. EguiPilot, devtools exe built first
   (`cargo build -p devpulselabs --features eguipilotai-devtools`), runs
   SEQUENTIAL:
   - `eguipilotai.exe test devpulse-windows tests/eguipilotai/chat-provider-tab.toml` → pass
   - `chat-provider-rows.toml` → pass (M1 unregressed)
   - `chat-provider-catalog.toml` → pass
   - `chat-peer-window.toml` → pass
6. End-to-end proof on the REAL exe: rebuild
   `target\release\devpulselabs.exe` FIRST (the `chat-native-proof`
   shim captures the release exe by window title — the M1 landmine),
   then `chat-native-proof.exe --out docs/proof/chat-provider-tab-native.png --state ai`
   at a REAL window size (`--window 1300x800`). The shot must show a
   connected provider's tab in the strip beside AI and Social.

## 7. Commit plan

Branch `feat/m3-connected-provider-tabs` off `main` at the tip after
#154 merges. One commit:

```
feat(chat): connected providers earn their own tab (POA&M M3)
```

`git add` — `crates/chat/window/src/lib.rs`,
`tests/eguipilotai/chat-provider-tab.toml`,
`docs/plans/chat-connected-provider-tabs.md`,
`docs/proof/chat-provider-tab-native.png`,
`docs/journal/2026-07-29-connected-provider-tabs.md`,
`docs/plans/poam-2026-07-29.md`, `docs/STATE.md`. Nothing else — the
untracked operator/parallel-session files in the tree are other
workstreams.

PR via `gh pr create`; merge ONLY on double-green
(`gh pr checks <n> --watch`, both lanes COMPLETED SUCCESS).
`gh pr list` first — the chat window crate is a collision magnet.

## 8. Evidence

1. `cargo test -p dpl-chat-window` → `test result: ok. 26 passed; 0
   failed` (22 pre-existing + 4 new pins: `only_activated_providers_
   earn_a_tab`, `a_provider_tab_selects_its_provider`, `selecting_a_
   domain_clears_the_provider_tab`, `signing_out_drops_the_tab_and_its_
   focus`).
2. `cargo clippy -p dpl-chat-window --all-targets -- -D warnings` →
   exit 0. No `#[allow]` added.
3. `git diff --cached --stat` → 4 files, exactly ONE under `crates/`
   (`crates/chat/window/src/lib.rs`). The M2 activation boundary held.
4. `cargo green` on the server1 lane → recorded on the PR.
5. EguiPilot (sequential): `chat-provider-tab` 15/15 steps, 0 failed;
   `chat-provider-rows`, `chat-provider-catalog`, `chat-assist-toggle`
   all PASS unregressed.
6. Proof: `docs/proof/chat-provider-tab-native.png`, real release exe,
   1300×800 — strip reads **AI · Codex · Gemini · Social 5**; Claude
   (Blocked) and Grok (Signed out) have no tab and remain in the roster.

### Deviations from the plan as written (all corrected in-session)

- **Step 5** assumed `PilotId::child()` returns a `PilotId`. It returns
  `PilotPath` (`crates/egui-pilot-ai/src/lib.rs:38`). `mode_button` was
  widened to `path: impl Into<PilotPath>` and its accent test moved from
  `id == AI_MODE_ID` to `path == SOCIAL_MODE_ID.path()` — same
  semantics, Social keeps violet.
- **Step 5** also planned `if compact { short_label } else { label }`.
  The proof shot showed the full label clipping to "ChatGPT / Co" inside
  the fixed 112px tab, so provider tabs now carry `short_label` ALWAYS
  with the full name on hover, and `compact` gained a
  `MODE_TAB_WIDTH * connected.len()` term so the search field yields
  rather than being crowded. Caught only by the experience gate.
- **Step 9** planned `hidden` expectations to prove Blocked/SignedOut
  providers have no tab. `hidden` resolves the locator first and fails
  with "locator did not match a live node" when the node was never
  minted; changed to `type = "count"`, `value = 0`.
- **Step 7** added `provider_tab_header` (not named in the plan) so the
  scoped rail keeps a way back and the provider's own row.
