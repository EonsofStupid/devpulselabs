# Coach research census

**Updated:** 2026-08-01  
**Program:** [`../../plans/coach-lab-experiment-program.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/coach-lab-experiment-program.md)  
**Import process:** [`docs-history-2026-10-12:docs/research/coach/import-process.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/research/coach/import-process.md)  
**Bleeding-edge control planes:** [`docs-history-2026-10-12:docs/research/coach/bleeding-edge-control-plane.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/research/coach/bleeding-edge-control-plane.md)  
**Verb catalog ADR (draft):** [`../../plans/coach-operator-verb-catalog-adr.md`](../../plans/coach-operator-verb-catalog-adr.md)  
**EguiPilot census (related):** [`../eguipilotai/README.md`](../eguipilotai/README.md)

Living index for Coach Lab experiments and industry grounding. Product
truth remains in plans + `docs-history-2026-10-12:docs/STATE.md`. This folder is **research +
experiment registry**, not the Operator hub UI.

## Experiment registry

| ID | Status | Hypothesis (one line) | Promotion |
|---|---|---|---|
| *(program)* | CAPTURED | Import Process + Lab rules | → bridge |
| CX-cursor-01 | **PASS** | Cursor overlay locate (`Search Ctrl+K`, `File`) | locate ≠ bridge |
| C2f | CHROME | Operator Import UI; fabric discarded after mint | incomplete |
| CX-bridge-01 | **PASS** | FabricSession resolve→invoke (viewport-independent) | hub wired |
| CX-control-plane-01 | **PASS** | Axum localhost + presence/effect gates | agent door |
| CX-events-01 | **PASS** | SSE geom events + move/resize/focus verbs | — |
| CX-verbs-p0-02 | **PASS** | open/close/type_text/wait_until/assert_state | — |
| CX-verbs-p1 | **PASS** | press_key/select/scroll | Teacher next · drag P2 |
| CX-teacher-agency-01 | PLANNED | Teacher = same resolve/invoke/mount rails | after events |
| CX-harvest-01 | PLANNED | Advanced Import when a11y thin | Advanced → maybe C2g |

**Conquest ladder:** Cursor (this seat) → Notepad (Windows) · Calculator =
regression seed only.

Register new rows **before** spike code. Manifest fields: see program §3.3.

## Industry grounding (2025–2026)

| Source | Takeaway for DevPULSE |
|---|---|
| [gptme computer-use](https://gptme.org/docs/howto/computer-use.html) | a11y-first → screenshot verify → coordinate last |
| [Terminator / t8r](https://t8r.tech/t/accessibility-api-computer-use-agents) | Rust desktop automation via AX; hybrid click router |
| [Geisterhand](https://github.com/Geisterhand-io/linux) / [gnome-ui-mcp](https://github.com/asattelmaier/gnome-ui-mcp) | Localhost HTTP / snapshot-uid peers — shapes only |
| [Odilia atspi](https://github.com/odilia-app/atspi) / [xa11y](https://github.com/xa11y/xa11y) | Rust event streams + cross-OS locators |
| [wdotool](https://github.com/cushycush/wdotool) + libei | Wayland-legal input when X11 falls away |
| ActionEngine / Murphy / AUTO-Explorer papers | Offline crawl → state-machine fabric; hours OK for dark apps |
| [OmniParser](https://github.com/microsoft/OmniParser) | Structured vision as **import** grounding, not lesson eyes |
| AccessKit + egui | In-app fabric for DevPULSE itself; not foreign-app crawler |
| Full table | [`docs-history-2026-10-12:docs/research/coach/bleeding-edge-control-plane.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/research/coach/bleeding-edge-control-plane.md) |

## Laws carried from research

1. Full-tree LLM dumps are an anti-pattern (context saturation).  
2. Online mint ≠ offline harvest.  
3. egui is lesson chrome + in-app AccessKit — not the foreign actuator.  
4. Stamp facts now for DuckDB later.

## Visuals

Operator look-target: [`../../ops/references/coach-operator-mock-c2c.html`](../../ops/references/coach-operator-mock-c2c.html).
