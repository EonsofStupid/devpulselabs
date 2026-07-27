# ChatGPT / Codex activation pass — execution plan

**Status:** EXECUTED — see §8
**Date:** 2026-07-27
**Ratified contract:** [`chat-codex-activation.md`](chat-codex-activation.md)
**Governing ADR:** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md) (activation-only slice clause)
**Pattern authority:** [`chat-grok-activation.md`](chat-grok-activation.md) (state machine, evidence, containment rules)

## 1. Context

DevPULSE Chat reaches official AI providers through their own subscription
clients. Grok is the first fully qualified activation (device-auth ceremony,
evidence ledger, bounded supervision, #91). The ratified Codex contract
(2026-07-25) commissions the same truthful activation slice for ChatGPT /
Codex: discover the depot-staged official `codex.exe`, ask it for non-secret
login status, own one bounded `codex login` path, and report `Ready` **only**
when `codex login status` says "Logged in using ChatGPT". API-key login is
visibly not Ready. Conversations, sessions, turns, and Send remain blocked
until the separate CH4 app-server pass — the ADR's activation-only clause is
explicit that activation success qualifies `SubscriptionAuth` and nothing
else.

Most of the slice already exists and runs on every real boot
(`dpl-chat-codex`): depot-only discovery, version band `[0.145.0, 0.146.0)`
(the staged `0.145.0-alpha.18` parses to `0.145.0` and qualifies), a
fail-closed `codex login status` probe with env-allowlisted quiet spawn, the
three-way ChatGPT/API-key/signed-out mapping, and a single-owner `codex
login` child wired to the catalog's "Log in with ChatGPT" button. This pass
closes the gaps between that and the contract:

1. the post-login `login status` verification runs **synchronously inside the
   UI-thread poll** (activation.rs:306–316) — it must become a bounded
   worker with a visible `Verifying` state, and it is currently untestable
   because the status probe has no injection seam;
2. Codex never records activation evidence (no `dpl-chat-accounts` rows, no
   attempt ids, no fabric `ProviderEvent`s — the activation trace node is
   dead for Codex);
3. `HostIntent::RefreshProvider` rejects Codex (chat.rs:685) and the
   coordinator has no `refresh_discovery`;
4. the window hardcodes "Verifying Grok connection…" (window lib.rs:2557)
   and Grok-specific ceremony copy for the `Verifying`/`SignedOut` states;
5. no EguiPilotAI scenario guards the real (non-authored) Codex activation
   state, and the contract requires proof that Codex and Grok expose
   independent states and neither falsely enables Send.

## 2. Current state (all read in-session, 2026-07-27, main @ 152436c)

- `crates/chat/providers/codex/src/activation.rs` (711 lines)
  - `CodexLoginStatus` is **private** (160–165).
  - `CodexActivationCoordinator { profile, state, launcher, child }`
    (233–239); `discover` (242), `from_parts(profile, state, launcher)`
    (253), `start_login` (278), `poll` (295) — the `Success` branch
    (307–316) synchronously calls `probe_login_status`, `fail` (322).
  - `probe_login_status` (458) shells `<exe> login status`, bounded to 512
    bytes, parsed by substring (`parse_login_status`, 470). No seam.
  - `login_command` (499) = literal `["login"]`, all stdio null.
  - `profiled_command` (509) = quiet-spawn + `env_clear()` + allowlist
    (522–558). `public_io_reason` (560).
  - Tests (628–710): version/status parsing, resolver ownership,
    single-owner login + drop reap, literal argument, staged discovery.
    Nothing tests the poll→status→Ready transition.
- `crates/chat/providers/codex/src/lib.rs`: `pub use` of activation items at
  lines 8–11; `PROVIDER = ProviderId("codex")`; manifest slice "CH4".
- `crates/app/src/chat.rs` (2497 lines)
  - `ChatProofMode` (44–73): `GrokLogin`/`GrokAuthorization` etc.; parse maps
    `"grok"`, `"grok-auth"`; seeding at 192–266 (GrokLogin selects+overlays,
    GrokAuthorization selects+forces `AwaitingAuthorization`).
  - `Shared` (79–109): `codex_activation` (83), `grok_attempt` (92),
    `activation_ledger` (96); no `codex_attempt`.
  - Boot (283–301): ledger opens, logs prior **Grok** evidence only.
  - `apply_host_intent` (640–806): `RefreshProvider` (684) rejects non-Grok;
    `OpenProviderSetup` (673) already handles Codex.
  - `activate_provider` (830–898): Codex arm (836–849) starts login with no
    attempt, no facts, no evidence; Grok arm (850–890) mints attempt,
    publishes `ActivationRequested`/`ClientStarted`, records failures.
  - Helpers: `grok_client_version` (974), `record_grok_failed` (981),
    `publish_grok_failure` (995), `ensure_grok_attempt` (1013).
  - `poll_provider_activations` (1028–1142): Codex poll (1032–1048) only
    projects state; Grok section publishes `DeviceCodeReceived`,
    `AuthorizationCompleted`, drives browser + verification.
- `crates/chat/window/src/lib.rs` (3998 lines)
  - `provider_activation_ceremony` (2364–2421): `Verifying` title
    "Verifying your Grok subscription" (2381), `SignedOut` detail "Use your
    existing xAI subscription through the official Grok device-auth flow."
    (2390). `AwaitingAuthorization` copy is xAI-specific (2377–2378) — that
    state is only ever produced by Grok.
  - `provider_activation_action` (2490–2634): `Verifying` branch renders
    "Verifying Grok connection…" (2557) at node
    `chat.provider.login/<key>`; `SignIn|Retry` recovery renders the login
    button emitting `ActivateProvider` (2584). No test pins these strings.
- `crates/fabric/src/lib.rs`: `ProviderEvent` (219–285) already has every
  variant needed (`ActivationRequested`, `ClientStarted`,
  `AuthorizationCompleted`, `Ready { client_version, auth_method: &'static
  str }`, `Failed { stage, class, retryable }`); `ProviderActivationStage`
  (163), `ProviderFailureClass` (184), `mint_provider_attempt` (441). **No
  fabric changes** — coach-signal exhaustive matches stay valid.
- `crates/chat/accounts/src/lib.rs`: provider-generic `ActivationLedger`
  (`record_ready[_for_attempt]` 124/133, `record_signed_out[_for_attempt]`
  153/162, `record_failed[_for_attempt]` 181/191, `get` 93). Only Grok
  writes today.
- `crates/app/src/bin/chat_native_proof.rs`: `--state` whitelist (48–59)
  and usage string (44); launches the app with `--chat-authored
  --chat-proof <state>`.
- Machine state: staged `dptools-deps/providers/codex/0.145.0-alpha.18/
  codex.exe` reports `codex-cli 0.145.0-alpha.18`; `codex login status` →
  "Logged in using ChatGPT" (exit 0). Grok readiness is persisted. So the
  real-state scenario on this machine sees **both** providers Ready, with
  Codex conversation actions disabled.

## 3. Target design

The Codex coordinator gains the same shape Grok's has, minus any runtime:

```text
SignedOut ──start_login()──> SigningIn ──login child exits 0──> Verifying
   ^                                                             │ bounded
   │                        (worker runs `codex login status`)   │ worker
   ├── status: signed out / API key ─────────────────────────────┤
   └── (Failed + Retry on launch/poll/status errors) <── errors ─┘
                         status: "Logged in using ChatGPT" ──> Ready
```

- `CodexLoginStatus` becomes public; new `CodexStatusProbe` /
  `CodexStatusRequest` traits parallel `CodexLoginLauncher` /
  `CodexLoginProcess`. The production probe runs `probe_login_status` on a
  named worker thread (`devpulse-codex-login-status`) delivering through a
  1-slot channel; `poll()` stays nonblocking and the UI thread never waits
  on a provider process.
- One-shot edges `take_login_completed()` / `take_verified_status()` let the
  app publish `AuthorizationCompleted` and record evidence exactly once.
- `refresh_discovery()` mirrors Grok's: rejected while a login child or
  verification is live, re-runs discovery (which re-probes status), reports
  whether the state changed.
- The app mirrors the Grok evidence pattern with `codex_attempt`,
  `codex_client_version`, `record_codex_failed`, `publish_codex_failure`.
  `auth_method` is `"chatgpt"` everywhere (Ready means ChatGPT-subscription
  auth; the verification instrument is always `codex login status`).
  Failure classes: `authorization_launch` (start), `authorization_process`
  (poll), `status_verification` (status probe), plus signed-out /
  ready rows from verification outcomes. Boot records the synchronous
  discovery probe's Ready/SignedOut outcome (no attempt id).
- The window's `Verifying` button and the `Verifying`/`SignedOut` ceremony
  copy become provider-generic via `descriptor.short_label`; node ids do
  not change, and the node *name* becomes the stable provider-neutral
  "Verifying provider connection".
- Scale story: Claude/Gemini activation slices later reuse the same window
  surfaces and app helpers by construction; only a provider crate + an
  `activate_provider` arm + helper trio is added per provider.

## 4. Changes, in execution order

### Step 1 — `crates/chat/providers/codex/src/activation.rs` (edit)

1a. Line 160: `enum CodexLoginStatus` → `pub enum CodexLoginStatus` with a
doc comment noting it is non-secret status classification.

1b. After the `OwnedChild` section (after line 231), add the status seam:

```rust
pub trait CodexStatusRequest: fmt::Debug + Send {
    /// Nonblocking. `None` while the probe is still running.
    fn try_status(&mut self) -> Option<Result<CodexLoginStatus, String>>;
}

pub trait CodexStatusProbe: fmt::Debug + Send {
    fn begin(&self, profile: &CodexRuntimeProfile) -> io::Result<Box<dyn CodexStatusRequest>>;
}

#[derive(Debug, Default)]
struct SystemCodexStatusProbe;

impl CodexStatusProbe for SystemCodexStatusProbe {
    fn begin(&self, profile: &CodexRuntimeProfile) -> io::Result<Box<dyn CodexStatusRequest>> {
        let profile = profile.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("devpulse-codex-login-status".to_owned())
            .spawn(move || {
                let result = probe_login_status(&profile).map_err(|error| error.to_string());
                let _ = sender.send(result);
            })?;
        Ok(Box::new(PendingStatus { receiver }))
    }
}

#[derive(Debug)]
struct PendingStatus {
    receiver: std::sync::mpsc::Receiver<Result<CodexLoginStatus, String>>,
}

impl CodexStatusRequest for PendingStatus {
    fn try_status(&mut self) -> Option<Result<CodexLoginStatus, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("the login status worker stopped".to_owned()))
            }
        }
    }
}
```

1c. Coordinator struct (233–239) gains fields; constructors updated:

```rust
pub struct CodexActivationCoordinator {
    discovery_root: Option<PathBuf>,
    profile: Result<CodexRuntimeProfile, CodexDiscoveryError>,
    state: ActivationState,
    launcher: Box<dyn CodexLoginLauncher>,
    status_probe: Box<dyn CodexStatusProbe>,
    child: Option<Box<dyn CodexLoginProcess>>,
    verification: Option<Box<dyn CodexStatusRequest>>,
    login_completed: bool,
    verified_status: Option<CodexLoginStatus>,
}
```

`discover` remembers `discovery_root: Some(path)` and installs the system
probe. `from_parts` gains a fourth parameter
`status_probe: Box<dyn CodexStatusProbe>` (`discovery_root: None`).

1d. New accessors mirroring Grok (`lib.rs` grok 365/371/399):

```rust
pub fn runtime_profile(&self) -> Option<CodexRuntimeProfile>   // profile.as_ref().ok().cloned()
pub fn take_login_completed(&mut self) -> bool                 // one-shot
pub fn take_verified_status(&mut self) -> Option<CodexLoginStatus>  // one-shot
pub fn refresh_discovery(&mut self) -> Result<bool, CodexActivationError>
```

`refresh_discovery` returns `Err(AlreadyRunning)` if `child` or
`verification` is live or `discovery_root` is `None`
(`Unavailable("the Codex coordinator has no discovery root")`); otherwise
re-runs `CodexRuntimeProfile::discover` + `activation_from_discovery`,
replaces `profile`/`state`, returns whether the visible state changed.

1e. `is_login_running` (274) → `self.child.is_some() || self.verification.is_some()`
(keeps the 100 ms repaint pump alive during verification).

1f. `start_login` (278): first guard becomes
`if self.child.is_some() || self.verification.is_some() { return Err(AlreadyRunning); }`;
on success also clears `login_completed`/`verified_status`.

1g. `poll` (295–320) rewritten (nonblocking in both phases):

```rust
pub fn poll(&mut self) -> Result<bool, CodexActivationError> {
    if let Some(child) = self.child.as_mut() {
        let exit = child
            .try_exit()
            .map_err(|error| CodexActivationError::PollFailed(public_io_reason(&error)))?;
        let Some(exit) = exit else { return Ok(false); };
        self.child = None;
        match exit {
            CodexLoginExit::Success => {
                self.login_completed = true;
                let profile = self
                    .profile
                    .as_ref()
                    .map_err(|error| CodexActivationError::StatusFailed(error.to_string()))?;
                let request = self
                    .status_probe
                    .begin(profile)
                    .map_err(|error| CodexActivationError::StatusFailed(public_io_reason(&error)))?;
                self.verification = Some(request);
                self.state = ActivationState::Verifying;
            }
            CodexLoginExit::Failed(code) => self.state = failed_state(code),
        }
        return Ok(true);
    }
    if let Some(request) = self.verification.as_mut() {
        let Some(result) = request.try_status() else { return Ok(false); };
        self.verification = None;
        let status =
            result.map_err(CodexActivationError::StatusFailed)?;
        self.verified_status = Some(status);
        self.state = activation_from_status(status);
        return Ok(true);
    }
    Ok(false)
}
```

(`Err` returns leave state repair to the caller's `fail(..)`, exactly as
today.)

1h. `fail` (322) also drops `self.verification = None;`.

1i. Tests: update the two `from_parts` call sites for the new parameter with
a `FakeStatusProbe`; add fakes
(`FakeStatusProbe { plan: Vec<Option<Result<CodexLoginStatus, String>>> }`
returning a `FakeStatusRequest` that pops the plan front per `try_status`)
and new tests:

- `login_success_verifies_status_before_ready` — exit `Success`, plan
  `[None, Some(Ok(ChatGpt))]`: first poll → `Verifying` +
  `take_login_completed()` once-true; second poll no-change; third →
  `Ready`, `take_verified_status() == Some(ChatGpt)` once.
- `api_key_status_is_not_ready` — plan `[Some(Ok(ApiKey))]` → `SignedOut`
  with `SignIn` recovery.
- `status_probe_failure_is_retryable` — plan `[Some(Err("boom"))]` → poll
  returns `Err(StatusFailed)`; after `fail(error)` the state is `Failed`
  with `Retry` recovery.
- `login_failure_skips_verification` — exit `Failed(Some(3))` → `Failed`,
  no verification, `take_login_completed()` false.
- `verification_blocks_duplicate_login` — while `Verifying`,
  `start_login()` → `AlreadyRunning`.
- `refresh_is_rejected_while_login_runs` — with a live fake child,
  `refresh_discovery()` → `AlreadyRunning`.

### Step 2 — `crates/chat/providers/codex/src/lib.rs` (edit)

Extend the `pub use activation::{…}` list (lines 8–11) with
`CodexLoginStatus, CodexStatusProbe, CodexStatusRequest`.

### Step 3 — `crates/app/src/chat.rs` (edit)

3a. `ChatProofMode`: add variant `CodexLogin` (after `ProviderHover`, 50);
`parse`: `"codex" => Some(Self::CodexLogin),`; seeding match (201–265) adds:

```rust
ChatProofMode::CodexLogin => {
    window
        .apply(dpl_chat_window::ChatWindowIntent::SelectProvider(
            dpl_chat_codex::PROVIDER,
        ))
        .expect("the authored Codex provider exists");
    window
        .set_provider_activation(
            dpl_chat_codex::PROVIDER,
            ActivationState::SignedOut {
                reason: "Log in with ChatGPT to activate subscription-backed Codex."
                    .to_owned(),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::SignIn,
                    "Log in with ChatGPT",
                )],
            },
        )
        .expect("the authored Codex sign-in ceremony is valid");
}
```

(add `RecoveryAction, RecoveryActionKind` to the `dpl_chat_core` import if
absent.)

3b. `Shared`: add `codex_attempt: Option<ActivationAttemptId>,` after
`grok_attempt` (92); initialize `codex_attempt: None,` beside it (315).

3c. Boot ledger block (283–301): after the Grok match, log Codex evidence
identically (`"loaded non-secret Codex activation evidence"` /
`"no prior Codex activation evidence"` / `"Codex activation evidence
unreadable"`). Then, still before `Shared` construction, record the boot
probe outcome when live:

```rust
if provider_runtime_enabled {
    if let Some(ledger) = activation_ledger.as_ref() {
        let version = codex_activation.runtime_profile().map_or_else(
            || "unknown".to_owned(),
            |profile| profile.version().to_string(),
        );
        let recorded = match codex_activation.activation() {
            ActivationState::Ready => ledger.record_ready(dpl_chat_codex::PROVIDER, &version, "chatgpt"),
            ActivationState::SignedOut { .. } => ledger.record_signed_out(dpl_chat_codex::PROVIDER, &version, "chatgpt"),
            _ => Ok(()),
        };
        if let Err(error) = recorded {
            tracing::warn!(%error, "Codex boot evidence could not be stored");
        }
    }
}
```

3d. `activate_provider` Codex arm (836–849) → mirror the Grok arm:

```rust
let activation = if provider == dpl_chat_codex::PROVIDER {
    let attempt = state.fabric.mint_provider_attempt();
    match state.codex_activation.start_login() {
        Ok(()) => {
            state.codex_attempt = Some(attempt);
            state
                .fabric
                .publish_provider(ProviderEvent::ActivationRequested { provider, attempt });
            state.fabric.publish_provider(ProviderEvent::ClientStarted {
                provider,
                attempt,
                client_version: codex_client_version(&state),
            });
        }
        Err(error) => {
            tracing::warn!(%error, "ChatGPT / Codex activation was rejected");
            if !matches!(&error, dpl_chat_codex::CodexActivationError::AlreadyRunning) {
                state.codex_attempt = Some(attempt);
                if matches!(
                    &error,
                    dpl_chat_codex::CodexActivationError::LaunchFailed(_)
                        | dpl_chat_codex::CodexActivationError::PollFailed(_)
                        | dpl_chat_codex::CodexActivationError::StatusFailed(_)
                ) {
                    state.codex_activation.fail(error);
                    record_codex_failed(&state, "authorization_launch");
                }
                publish_codex_failure(
                    &state,
                    ProviderActivationStage::Launch,
                    ProviderFailureClass::Launch,
                    true,
                );
            }
        }
    }
    state.codex_activation.activation().clone()
} else if provider == dpl_chat_grok::PROVIDER {
```

3e. New helpers after `publish_grok_failure` (1011):

```rust
fn codex_client_version(state: &Shared) -> String {
    state.codex_activation.runtime_profile().map_or_else(
        || "unknown".to_owned(),
        |profile| profile.version().to_string(),
    )
}

fn record_codex_failed(state: &Shared, failure_class: &str) {
    if let Some(ledger) = state.activation_ledger.as_ref() {
        if let Err(error) = ledger.record_failed_for_attempt(
            dpl_chat_codex::PROVIDER,
            state.codex_attempt.map(|attempt| attempt.0),
            &codex_client_version(state),
            "chatgpt",
            failure_class,
        ) {
            tracing::warn!(%error, "Codex failure evidence could not be stored");
        }
    }
}

fn publish_codex_failure(
    state: &Shared,
    stage: ProviderActivationStage,
    class: ProviderFailureClass,
    retryable: bool,
) {
    let Some(attempt) = state.codex_attempt else {
        return;
    };
    state.fabric.publish_provider(ProviderEvent::Failed {
        provider: dpl_chat_codex::PROVIDER,
        attempt,
        stage,
        class,
        retryable,
    });
}
```

3f. `poll_provider_activations` Codex section (1032–1048) → extended:

```rust
let codex_changed = match state.codex_activation.poll() {
    Ok(changed) => changed,
    Err(error) => {
        tracing::warn!(%error, "ChatGPT / Codex activation polling failed");
        let failure_class = if matches!(
            &error,
            dpl_chat_codex::CodexActivationError::StatusFailed(_)
        ) {
            "status_verification"
        } else {
            "authorization_process"
        };
        state.codex_activation.fail(error);
        record_codex_failed(state, failure_class);
        publish_codex_failure(
            state,
            ProviderActivationStage::Authorization,
            ProviderFailureClass::Authorization,
            true,
        );
        true
    }
};
if codex_changed {
    // (existing projection block unchanged)
}
if state.codex_activation.take_login_completed() {
    if let Some(attempt) = state.codex_attempt {
        state
            .fabric
            .publish_provider(ProviderEvent::AuthorizationCompleted {
                provider: dpl_chat_codex::PROVIDER,
                attempt,
            });
    }
}
if let Some(status) = state.codex_activation.take_verified_status() {
    let version = codex_client_version(state);
    match status {
        dpl_chat_codex::CodexLoginStatus::ChatGpt => {
            if let Some(ledger) = state.activation_ledger.as_ref() {
                if let Err(error) = ledger.record_ready_for_attempt(
                    dpl_chat_codex::PROVIDER,
                    state.codex_attempt.map(|attempt| attempt.0),
                    &version,
                    "chatgpt",
                ) {
                    tracing::warn!(%error, "Codex ready evidence could not be stored");
                }
            }
            if let Some(attempt) = state.codex_attempt {
                state.fabric.publish_provider(ProviderEvent::Ready {
                    provider: dpl_chat_codex::PROVIDER,
                    attempt,
                    client_version: version,
                    auth_method: "chatgpt",
                });
            }
        }
        dpl_chat_codex::CodexLoginStatus::ApiKey
        | dpl_chat_codex::CodexLoginStatus::SignedOut => {
            if let Some(ledger) = state.activation_ledger.as_ref() {
                if let Err(error) = ledger.record_signed_out_for_attempt(
                    dpl_chat_codex::PROVIDER,
                    state.codex_attempt.map(|attempt| attempt.0),
                    &version,
                    "chatgpt",
                ) {
                    tracing::warn!(%error, "Codex signed-out evidence could not be stored");
                }
            }
            publish_codex_failure(
                state,
                ProviderActivationStage::Authorization,
                ProviderFailureClass::Authorization,
                false,
            );
        }
    }
}
```

3g. `RefreshProvider` arm (684–709) → provider dispatch:

```rust
HostIntent::RefreshProvider(provider) => {
    let mut state = lock(shared);
    if provider == dpl_chat_codex::PROVIDER {
        match state.codex_activation.refresh_discovery() {
            Ok(_) => {
                let activation = state.codex_activation.activation().clone();
                if let Err(error) = state.window.set_provider_activation(provider, activation) {
                    tracing::warn!(%error, "Codex rediscovery state could not be projected");
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Codex rediscovery was rejected");
            }
        }
    } else if provider == dpl_chat_grok::PROVIDER {
        // (existing Grok body unchanged)
    } else {
        tracing::warn!(%provider, "provider rediscovery is not qualified");
        return;
    }
    ctx.request_repaint();
}
```

### Step 4 — `crates/chat/window/src/lib.rs` (edit)

4a. `provider_activation_action` Verifying branch (2556–2567):

```rust
if matches!(provider.registration.activation, ActivationState::Verifying) {
    let label = format!(
        "Verifying {} connection…",
        provider.registration.manifest.descriptor.short_label
    );
    let response = ui.add_enabled(false, egui::Button::new(&label));
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            PROVIDER_LOGIN_ID.child(&PilotKey::new(provider_id.0)),
            Role::Button,
            "Verifying provider connection",
        ),
    );
    return;
}
```

4b. Ceremony `Verifying` copy (2380–2383):

```rust
ActivationState::Verifying => (
    format!("Verifying your {} subscription", descriptor.short_label),
    "Authorization completed. DevPULSE is verifying the provider's subscription session.",
),
```

4c. Ceremony `SignedOut` copy (2388–2391):

```rust
ActivationState::SignedOut { .. } => (
    format!("Connect {}", descriptor.short_label),
    "Sign in through the official provider client. Your subscription and credentials stay provider-owned.",
),
```

(`AwaitingAuthorization` copy stays xAI-specific: only Grok produces device
prompts.)

### Step 5 — `crates/app/src/bin/chat_native_proof.rs` (edit)

Add `"codex"` to the `--state` whitelist (`matches!` at 48–59) and to the
usage string (44).

### Step 6 — `tests/eguipilotai/chat-codex-persisted-ready.toml` (create)

Operator-state proof, `fixture = "chat-peer"` (real state; this machine is
ChatGPT-authenticated): open + wait 750 + focus; click
`chat.primary/chat.sidebar.providers`; expect
`chat.primary/chat.provider.catalog` visible; expect value `Ready` on
`chat.primary/chat.sidebar.providers/codex` AND on
`chat.primary/chat.sidebar.providers/grok` (independent real states); click
the codex row; expect `chat.primary/chat.provider.new-chat/sidebar`
**disabled** and `chat.primary/chat.composer.send` **disabled** (Ready
without conversation — the "no false Send" contract clause); close viewport.

### Step 7 — docs

- `docs/plans/chat-codex-activation.md`: status line → implementation
  landed, append a "2026-07-27 local evidence" section (mirroring Grok's).
- This file §8 filled with the real command outputs.
- `docs/STATE.md` tip in the same PR.

## 5. Boundaries

- 🚫 **Never:** qualify `StreamTurns`/`StartSession`/`ResumeSession` for
  Codex or un-gate `NewSession`/`SendTurn`/`CancelTurn`/
  `ResolveProviderPermission`/`OpenProviderAuthorization` for Codex (ADR:
  activation-only); read/copy/parse any Codex credential file; store codes,
  URLs, or tokens in the ledger; add `ProviderEvent` variants (coach-signal
  exhaustive matches); touch the Grok runtime/retry code; touch operator
  credential state (`codex logout` is forbidden — the login ceremony
  remains an explicit operator gate); `/content`; `#[allow]`.
- ⚠️ **Ask first:** widening the Codex version band; changing the authored
  testkit Codex fixture; altering `AwaitingAuthorization` xAI copy; any new
  crate dependency.
- **Out of scope:** CH4 app-server conversations, Claude/Gemini slices,
  `dpl-chat-profiles` wiring for Codex, making the boot-time synchronous
  status probe async (pre-UI, bounded, existing behavior).

## 6. Verification

1. `cargo test -p dpl-chat-codex` — exit 0; the six new tests listed in
   step 1i pass.
2. `cargo green` — exit 0 (fmt, docs-status, spawn audit, clippy `-D
   warnings`, full workspace tests, release build; release exe mtime moves).
3. Build devtools exe (`cargo build -p devpulselabs --features
   eguipilotai-devtools`), then sequential EguiPilotAI runs, all
   `"passed": true`:
   `chat-codex-persisted-ready` (new), `chat-provider-catalog`,
   `chat-steam-social`, `chat-peer-window`, `chat-grok-persisted-ready`,
   `chat-grok-device-authorization`.
4. Native proof: `target\debug\chat_native_proof.exe --out
   proof-chat-codex-activation.png --state codex --app
   target\debug\devpulselabs.exe` exits 0 and the PNG shows the "Connect
   Codex" ceremony with the "Log in with ChatGPT" action (visual check).
5. Evidence persistence: launch the real exe once (Codex Ready via boot
   probe), relaunch with `RUST_LOG=devpulselabs=info` and observe
   `loaded non-secret Codex activation evidence` with `state=Ready`,
   `method=chatgpt`.
6. Explicit operator gate (NOT this pass, same as Grok): sign the official
   Codex client out, click "Log in with ChatGPT" in Chat, complete OpenAI
   browser auth, watch SigningIn → Verifying → Ready with the activation
   trace populated.

## 7. Commit plan

Branch `feat/chat-codex-activation` → one commit:

```
feat(chat): qualify ChatGPT/Codex activation to the Grok contract

Codex post-login verification moves off the UI thread behind a status-probe
seam with a visible Verifying state; activation evidence (attempts, Turso
ledger rows, fabric facts) now mirrors Grok; RefreshProvider and the
activation trace work for Codex; window ceremony copy is provider-generic.
Ready still qualifies SubscriptionAuth only — conversations stay CH4-gated.
```

`git add`: the six files in steps 1–5 plus
`tests/eguipilotai/chat-codex-persisted-ready.toml`,
`docs/plans/chat-codex-activation.md`,
`docs/plans/chat-codex-activation-pass.md`, `docs/STATE.md`.
Push (pre-push hook runs the gate), `gh pr create`, squash-merge, rebuild
release exe on `main`.

## 8. Evidence (2026-07-27)

- `cargo test -p dpl-chat-codex` — 15 passed, 0 failed (the six new tests
  from step 1i included).
- `cargo check --workspace` — exit 0. The devtools build
  (`--features eguipilotai-devtools`) initially failed with
  `E0004: ProviderEvent::ProfileConfigured not covered` at
  `coach_signals.rs:148` — a latent semantic conflict between #94 (new
  fabric variant) and #93 (exhaustive coach matches) that merged
  independently green; the gate does not build the devtools feature.
  **Deviation from plan (reported):** fixed here as a required enabling
  repair — `providers.profile-configured` added to the match, const table,
  `COARSE`, and the test's every-fact list. All 7 coach-signal contract
  tests pass.
- EguiPilotAI, real Windows/wgpu process, sequential:
  `chat-codex-persisted-ready` **passed** (catalog shows Codex `Ready` and
  Grok `Ready` independently; with Codex selected, `chat.provider.new-chat/
  sidebar` and `chat.composer.send` are disabled — Ready without
  conversation, per the ADR activation-only clause). Regression sweep all
  passed: `chat-provider-catalog`, `chat-steam-social`, `chat-peer-window`,
  `chat-grok-persisted-ready`, `chat-grok-device-authorization`.
- Native proof `proof-chat-codex-activation.png` captured via
  `chat-native-proof.exe --state codex`: "Connect Codex" ceremony,
  "Log in with ChatGPT" single action, Send disabled.
- Evidence persistence: fresh boot logs
  `loaded non-secret Codex activation evidence state=Ready version=0.145.0
  method=chatgpt` (alongside Grok's `state=Ready version=0.2.112
  method=cached_token`).
- `cargo green` enforced by the pre-push hook on the branch push.
- Operator gate outstanding (by design): real sign-out → "Log in with
  ChatGPT" → OpenAI browser auth → SigningIn → Verifying → Ready with a
  populated activation trace.
