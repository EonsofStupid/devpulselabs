# Estate → seam wire — durable route + WiredFRONT lastComp

## 1. Context

`dpl-estate` landed (#65) with zero consumers. The agent seam (`dpl-seam`)
already serves `GET /state` / `POST /command` / `GET /signals`, but
`last_route` is process-local memory and WiredFRONT's reopen pointer lives
only in `composer-state.json`. The ADR
(`docs/design/estate-and-agent-seam.md`) says GET `/state` should read from
the estate. This pass is the smallest honest join: open the estate at Ring 0,
persist shell route + dual-write `last_comp`, expose both on `/state`.

## 2. Current state (read in-session)

- `crates/estate` — `Estate::{open,put_json,get_json,persist}`, `Tree::State`.
- `crates/seam` — `SeamState { last_route, … }`; `get_state` returns
  `{version, route, hubs}` only; `spawn(fabric, registry, root, port)`.
- `crates/composer` — `remember_last` → `state::save` under `content_root()`
  (kept this pass); no estate handle.
- `crates/app` — opens seam after hubs; default boot route is
  `wiredfront/viziwiz` when no `--route`.
- ADR-014 — userdata is Ring 0; estate opens at `userdata_root()/estate`.

## 3. Target design

- Open `Estate` once: `userdata_root().join("estate")`. Open failure → warn,
  continue with `estate: false` (never crash).
- `Arc<Estate>` shared into seam + `Composer` (via `WiredfrontHub::wire_estate`).
- Keys on `Tree::State`: `shell/route` (`"hub/sub"`), `wiredfront/last-comp`
  (content-rel path).
- On `RouteChanged`: write + persist `shell/route`.
- On seam construct: hydrate `last_route` only via `Registry::resolve`.
- App: if no `--route`, navigate to hydrated route when valid; else default.
- `GET /state` adds `"estate": bool`, `"lastComp": string|null`.
- Keep `composer-state.json` dual-write this pass.

## 4. Changes, in execution order

1. Plan + ADR honesty amend (§2 DONE = these fields only).
2. `dpl-estate` pub key consts; `dpl-seam` depends on estate; `SeamState` holds
   `Option<Arc<Estate>>`; hydrate/persist; `GET /state` fields; `spawn` arg.
3. `dpl-composer` + wiredfront: `wire_estate`, dual-write in `remember_last`.
4. App: open estate, wire, spawn, restore boot route.
5. Tests + EguiPilot scenario + two-boot curl drill.
6. `cargo green` → PR → squash-merge → tip STATE.

## 5. Boundaries

- Out: `Tree::Docs` / `Agents` / `Manifest`; selection/undo; Workbench session;
  deleting `composer-state.json`; new HTTP commands; non-localhost bind.
- Never invent routes — registry resolve or discard.
- Do not "fix" `remember_last`'s content_root JSON path this pass.

## 6. Verification

1. `cargo test -p dpl-seam -p dpl-estate -p dpl-composer` (touched) green.
2. `env -u CARGO_TARGET_DIR cargo green`.
3. EguiPilot `devpulse-cloud` scenario green; two-boot curl drill: navigate
   (+ save-comp) → kill → boot → `GET /state` matches route and/or lastComp.

## 7. Commit plan

Branch `feat/estate-seam-wire`. One feature commit (+ docs tip after merge).

## 8. Evidence

- `cargo test -p dpl-seam -p dpl-estate -p dpl-composer --lib` green
  (seam hydrate/persist/state fields; composer dual-write).
- `scripts/estate-seam-restart-drill.sh` PASS — boot A navigate to
  `wiredfront/library` → kill → boot B (no `--route`) → `GET /state`
  route=`wiredfront/library`, `estate=true`.
- EguiPilotAI `devpulse-cloud` scenario
  `tests/eguipilotai/estate-seam-wire.toml` fixture `wiredfront-editor`
  **passed** — run
  `target/eguipilotai/runs/run-823214-18c5ff0d66d54175`; proof
  `docs/proof/estate-wiredfront-editor.png`.
- Spawn-after-navigate + seed `last_route` closes the broadcast race
  (RouteChanged before fold subscribe).
