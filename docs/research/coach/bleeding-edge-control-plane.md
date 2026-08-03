# Research — bleeding-edge desktop control planes (2026-08-01)

**Question:** What installable / downloadable tools help DevPULSE build an
app-agnostic operator framework (the 101 verbs) for accurate dynamic teaching?

**Position (high confidence):** Steal peer **HTTP/verb shapes** and event
models; keep **our** fabric identity + localhost Axum daemon as product law.
Primary Linux event stack = Odilia `atspi` (already in workspace). Do not
adopt third-party MCP as the production Coach runtime.

**Related ADR (draft):** [`../../plans/coach-operator-verb-catalog-adr.md`](../../plans/coach-operator-verb-catalog-adr.md)

## Premises checked

| Premise | Verdict |
|---|---|
| Screenshots are required for teaching | **False** when AT-SPI/UIA works — peers + our live capture agree |
| AT-SPI alone proves a window is real | **False** — need mapped client + live pid (control-plane ADR) |
| AT-SPI `window:move`/`resize` are enough | **False on this seat** — X11 ConfigureNotify/geom poll heard; those AT-SPI names did not |
| MCP daemons should become Coach | **False** — optional thin adapter later; product owns catalog |

## Seat install (done 2026-08-01)

```bash
sudo apt-get install -y accerciser xdotool wmctrl python3-dogtail python3-pyatspi
gsettings set org.gnome.desktop.interface toolkit-accessibility true
```

Verified: `accerciser`, `xdotool`, `wmctrl` on PATH; `dogtail` + GI `Atspi`
import OK; `toolkit-accessibility` → `true`. `libei1` was already present.

## Live signal proof (same day)

Scripted Calculator move/resize + daemon invoke while listening:

| Signal | Heard |
|---|---|
| X11 geom poll move/resize | yes |
| X11 `ConfigureNotify` | yes |
| AT-SPI `object:bounds-changed` | yes (button reflow) |
| AT-SPI `object:text-changed:insert` | yes (`9`) |
| Daemon `effect_observed` | yes (`7` → `79`) |
| AT-SPI `window:move` / `window:resize` | no |

Artifacts: `/tmp/coach-signal-capture3/summary.json` (operator machine).

## Industry map

| Tool | Link | Role for us |
|---|---|---|
| Odilia `atspi` | https://github.com/odilia-app/atspi | **Ship** event_stream in daemon |
| Accerciser / dogtail | apt | Operator tree inspect + scripts |
| xdotool / wmctrl | apt | X11 stimuli / window verbs research |
| Geisterhand Linux | https://github.com/Geisterhand-io/linux | Peer HTTP `:7676` shape (tree/click/wait) |
| gnome-ui-mcp | https://github.com/asattelmaier/gnome-ui-mcp | Snapshot → stable uid → effect verify |
| uia-x | https://github.com/doucej/uia-x | Cross-OS MCP surface (Win UIA later) |
| xa11y | https://github.com/xa11y/xa11y | Cross-platform Rust/Py locators + events |
| kde-mcp / kwin-mcp | GitHub | Plasma/Wayland reference only |
| wdotool + libei | https://github.com/cushycush/wdotool | Wayland-legal input fallback |
| AutoKaam AT-SPI notes | https://autokaam.com/tutorials/at-spi2-ai-agent-native-linux-gui-automation/ | find→act→verify doctrine |

## What we take / what we refuse

**Take:** find→act→verify; uid/stale rejection; wait helpers; HTTP localhost
daemon; SSE/event subscribe; effect-bearing responses.

**Refuse:** screenshot-as-primary loop; coordinate click as address space;
binding non-loopback; replacing fabric with opaque MCP refs; depending on
KDE/GNOME-only portals in the Coach core.

## Open questions

1. Does xa11y’s event API beat hand-rolled Odilia streams enough to vendor
   patterns (not the crate) into `/events`?
2. Geisterhand maturity — clone to `/tmp` and bake verb names, or skip?
3. Windows: FlaUI / uia-x / desktop-touch-mcp — which event observables for
   Notepad conquest?

## Next experiment

**CX-events-01** (after verb ADR ratified): daemon `GET /events` +
`move`/`resize`/`focus` verbs; operator drags Calculator; Teacher sees
normalized facts without screenshots.
