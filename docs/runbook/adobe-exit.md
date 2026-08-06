# Runbook — the Adobe exit (while the subscription is live)

Operator-run migration of purchased AE templates, presets, .mogrts, and
PS/AI assets to open formats the WiredFRONT editor consumes. Researched
2026-08-06 (sources at bottom). Companion program plan:
`docs/plans/media-experience-program.md` Arc C.

The organizing fact: **`.aep`, `.ffx`, and the `.aep` inside every `.mogrt`
are dead formats the day the subscription ends** — nothing outside AE ever
re-renders or re-edits them (only partial metadata parsers exist). PSD and
AI merely *degrade* (GIMP/Inkscape keep reading them). So the clock-critical
work is the AE material; images are P2.

## Priority matrix

| # | Asset class | Export target | How | Priority |
|---|---|---|---|---|
| 1 | Cloud-stored files + CC fonts inventory | local disk; font list → licensed copies or text-to-outlines | CC app sync-down; font audit in AE/PS | **P0 — 30-day evacuation clock at cancellation; fonts deactivate and silently break layouts** |
| 2 | .mogrt library | extracted .aep + `definition.json` per template | unzip ×2 (below) — no AE needed, fully scriptable | **P0 — cheap, unlocks rows 3-4** |
| 3 | .aep + extracted projects → rendered archive | ProRes 4444 .mov (+alpha) + H.264 proxy + thumbs; PNG sequences for overlay elements; 4K only where authored | queue-all-comps.jsx → aerender loop (below) | **P0 — the long pole; start now, runs unattended** |
| 4 | Shape/vector-friendly comps → Lottie .json | Bodymovin/LottieFiles panel | manual GUI session per project (no headless mode exists); panel batches all comps per session | **P1 — operator-hours; triage to templates worth RE-ANIMATING in WiredFRONT (rendered media covers mere playback)** |
| 5 | .ffx presets | (a) rendered sample applications (b) property-tree JSON dump | apply in AE + row-3 render; ExtendScript property walker | **P1 — confirmed: no .ffx converter exists anywhere; the recipe dies with AE** |
| 6 | .psd | keep layered .psd + per-layer PNGs for key files + flat PNG + Smart-Object contents exported | Photoshop Export Layers to Files / Image Processor now; GIMP reads the rest later | P2 — degrades, doesn't die |
| 7 | .ai / .eps | SVG exported FROM Illustrator (best fidelity) + keep source | MultiExporter.jsx / ExportOptionsSVG batch; post-subscription fallback: `inkscape in.ai --export-type=svg` | P2 |
| 8 | Provenance | per-asset note (source, license, date) in the library index | ongoing | P2 |

## Row 2 — .mogrt extraction (scriptable today)

A `.mogrt` is a zip containing a `.aegraphic` which is itself a zip
containing the `.aep` + assets + `definition.json` (the Essential Graphics
control schema — archive it, it documents the template's exposed knobs):

```powershell
Copy-Item x.mogrt x.zip; Expand-Archive x.zip x\
# inside: definition.json, thumbnail, *.aegraphic
Copy-Item x\*.aegraphic y.zip; Expand-Archive y.zip y\   # → the .aep + footage
```

(AE's File → Open Project also accepts a .mogrt directly.) Caveat:
Premiere-authored .mogrts contain Premiere sequence data, not an .aep —
those render out of Premiere instead.

## Row 3 — the aerender loop

One-time setup in the AE GUI (Edit → Templates): create two output-module
templates — `PNG_Alpha` (PNG sequence, RGB+Alpha) and `ProRes4444_Alpha`
(ProRes 4444 encodes natively on Windows since 2018, alpha included,
ffmpeg-decodable — the right mezzanine for our pipeline).

Queue every comp (ExtendScript, run via `AfterFX.exe -r queue-all.jsx`):

```jsx
var rq = app.project.renderQueue;
for (var i = 1; i <= app.project.numItems; i++) {
  var it = app.project.item(i);
  if (it instanceof CompItem) {
    var rqi = rq.items.add(it);
    rqi.applyTemplate("Best Settings");
    var om = rqi.outputModule(1);
    om.applyTemplate("ProRes4444_Alpha");
    om.file = new File("X:/archive/" + it.name.replace(/[^\w-]/g,"_") + ".mov");
  }
}
app.project.save();
```

Then chew the queue headless (no `-comp` ⇒ renders every queued item):

```
"C:\Program Files\Adobe\Adobe After Effects <ver>\Support Files\aerender.exe"
  -project "X:\proj.aep"
```

Per comp, archive: ProRes 4444 at native size (always) · H.264 1080p proxy
(always — this is what the WiredFRONT library indexes for browse) · PNG
sequence with alpha (overlay/element comps) · frame-0 + mid-frame PNG
stills (thumbnails). Don't upscale raster comps to 4K — wasted disk.
Runs are SEQUENTIAL per the proof-shot law (shared AE caches).

## Row 4 — Lottie rules

- Export via the LottieFiles/Bodymovin AE panel; it lists ALL comps in the
  open project — tick everything, one destination, one session per .aep.
- **Bake expressions to keyframes** (the panel has a convert step). ThorVG's
  expression support is experimental — baked JSON is the only reliable form
  for WiredFRONT playback.
- Text → Glyphs (shapes); avoid embedded raster payloads.
- What survives: shape layers, transforms, parenting, additive masks,
  precomps. What doesn't: effects (almost all), layer styles, blend modes,
  luma track mattes, 3D/camera, time-remap (partial).
- Archive as plain `.json` (the primitive; ThorVG consumes it; bundle to
  `.lottie` later if ever needed).

## Row 5 — .ffx capture

Confirmed unconvertible (no tool exists, not even to Premiere's format).
Capture = apply the preset to a representative comp/layer (text presets: to
sample text at several sizes), then row-3 render with alpha, AND dump the
parameter record: an ExtendScript walker over `layer.property(...)`
recursively into JSON preserves values/keyframes/expressions as
documentation even though nothing can re-apply it outside AE.

## Licensing (brief; not legal advice)

- Envato: modification for your own end products is licensed; prohibited is
  redistributing items (or conversions of them) as extractable assets.
  Format-shifting for the operator's own pipeline = ordinary licensed use;
  never ship the converted template library where users can pull assets out.
- Adobe Stock standard license is **perpetual** (survives cancellation) and
  includes modify; same no-redistribution line.

## What dies vs degrades at cancellation

| Format | Status after |
|---|---|
| .aep / .ffx / .mogrt-inner-aep | dead — render/export NOW |
| .psd | readable (GIMP/Krita): layers/masks yes; Smart Objects rasterize, adjustment layers/styles degrade; text needs the fonts |
| .ai | readable via embedded PDF stream (Inkscape), degraded — export SVG from Illustrator now |
| .eps | fine forever (Ghostscript/Inkscape) |
| CC fonts | deactivate — audit and replace/outline NOW |
| cloud documents | 30 days, 5 GB cap — pull local NOW |
| Lottie .json, SVG, PNG/EXR, ProRes/H.264, definition.json | open — safe forever |

## Sources

airbnb/lottie supported-features (github.com/airbnb/lottie
after-effects.md) · LottieFiles AE plugin docs (docs.lottiefiles.com) ·
bodymovin-extension issue #41 (no headless batch) · dotlottie.io/spec ·
Adobe automated-rendering doc (helpx.adobe.com after-effects
automated-rendering-network-rendering) · AE scripting OutputModule
(ae-scripting.docsforadobe.dev) · ProRes-on-Windows (cined.com 2018-12) ·
.mogrt teardown (dastips.wordpress.com 2018-03-03) · .ffx no-converter
(community.adobe.com thread 9000555) · GIMP PSD compat (gimp.cc,
gimp.org/release-notes/gimp-3.0.html) · Illustrator ExportOptionsSVG
(ai-scripting.docsforadobe.dev) + MultiExporter.jsx (gist TomByrne) ·
Inkscape CLI (lists.inkscape.org) · Envato license FAQ + prohibited usage
(help.elements.envato.com) · Adobe Stock product terms PDF (adobe.com
legal, 2026-01-16) · account-access-after-cancellation (helpx.adobe.com) ·
aep parsers inventory: boltframe/aftereffects-aep-parser (Go),
forticheprod/aep_parser (Python/Kaitai — the reusable grammar),
actumn/aepx.js (dead format).
