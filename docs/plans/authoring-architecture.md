# The authoring architecture — seed slides, presets, click-to-edit

STATUS: DRAFT for operator ratification (2026-07-22). This is the alignment
doc — the "how the really smart people build this" answer, mapped onto what
already exists. No code moves until ratified.

## The operator's requirement (verbatim)

> "under workbench we need to be truly setting our default seed styling
> slides for use that the editor will allow me to easily click, replace
> font, edit existing, I should be able to click the actual background of
> the slide and add a mood where two gradients blend and move subtle and
> then add the pulse animation. This edits the colocated file. ... show me
> the platform aligned industry way to do this modular where we have the
> generated styling that is well organized where we then have presets
> pulled from all of that well organized auto generated styling that will
> allow me to easily create preset motion, interaction ... each slide is
> its own folder where it has the structure, content"

## The industry pattern, named

What Figma, Keynote, and Rive all converge on is a four-layer stack, and we
already own the bottom two:

```
4. EDITOR      a view over the document — click → select → swap a REF →
               serialize back to the colocated file. Never generates code.
3. DOCUMENT    slide-as-folder: structure + content + style REFS (never
               values). serde schema. The single source of truth.
2. PRESETS     named compositions of registry refs — DATA over layer 1.
               tokens/presets.json (the 6th source) → generated registry.
1. TOKENS      hues / fluid / motion / moods / fonts — authored JSON →
               build.rs → generated registries.  ✅ ALREADY BUILT
0. RUNTIME     egui + wgpu paints by resolving refs through registries.
               ✅ ALREADY BUILT (needs mood-blend + ambience layering)
```

The load-bearing rule making click-to-edit trivial is **refs, never
values**: a slide file never contains a hex color, a pixel size, or a font
path — it names ids (`"font": "orbitron"`, `"mood": ["ember-drift",
"deep-lagoon"]`, `"ambience": "pulse"`). Then:

- "replace font" = the editor swaps one string and re-serializes;
- every picker is a FREE enumeration of an existing registry (fonts wall,
  moods wall, motion effects — the LIBRARIES pay off here);
- restyling globally = editing a token, and every slide follows;
- the renderer, the editor stage, the deck, and thumbnails are ONE code
  path resolving the same refs.

## Layer 3 — the document: each slide is its own folder

```
seeds/slides/<slide-id>/            (repo — the default styling seeds)
  slide.json                        structure + refs (the editable truth)
  <media assets alongside>          copied in, referenced by filename
content/episodes/<ep>/slides/<id>/  (operator data — same shape, cloned
                                     from a seed, never mutated in place
                                     except BY the editor's explicit save)
```

`slide.json` (serde, versioned):

```json
{
  "v": 1,
  "template": "title",
  "background": {
    "kind": "mood-blend",
    "moods": ["ember-drift", "deep-lagoon"],
    "motion": "blend-drift",
    "ambience": "pulse"
  },
  "blocks": [
    { "id": "eyebrow", "kind": "text", "text": "Echoes of Sentience",
      "font": "sharetechmono", "size": "fs-eyebrow", "ink": "tone" },
    { "id": "title",   "kind": "text", "text": "The stack, laid bare",
      "font": "orbitron", "size": "fs-display", "ink": "strong" }
  ],
  "enter": "rise"
}
```

Everything in quotes on the right side of a style key is a REGISTRY ID or
PRESET ID. rustc + a doc validator enforce that unknown ids degrade to
labeled plates (Law: degrade, never crash) and the doctor flags them.

### The look cascade (W3b — ratified 2026-07-22)

An episode carries an authored **`theme.json`** — the show's trait, refs
only (default font, ink, background, enter). Slide styling fields are
**sparse**: absent = inherit the look; present = a deliberate override
(`enter: "none"` = explicitly static; `background {"kind":"none"}` =
deliberately bare). `dpl_doc::resolve(slide, look)` is the whole cascade —
**exactly two levels, never more** (the slide-master pattern; PPTX's
three-deep chain is the cautionary tale). Standalone `seeds/slides/*`
stay FULLY specified by design — they are self-contained starters with
no show to inherit from; episodes without a theme.json resolve against
`Theme::fallback()`.

**The folder contract** (doctor-enforced — strays, orphan slide folders,
leftover save transients, and bad look refs are all flagged):

```
seeds/episodes/<id>/
  episode.json      structure: chapters → slide subfolder names
  theme.json         the show's authored trait (optional; refs only)
  <slide>/
    slide.json      structure + content + sparse style overrides
    (reserved: ink.json D3, notes.md D4, assets/ X-track)
```

A new concern claims a reserved filename here first; nothing squats. In
the editor, trays show the RESOLVED value and a "Show look" chip clears
an override back to inheritance — restyling a whole show is one edit to
theme.json.

**The mood-blend background** (the operator's spec): two authored moods,
their gradients crossfaded and slowly panned by an authored motion effect
(`blend-drift` — new entry in tokens/motion.json, subtle by construction),
with an optional ambience (Pulse/Drift/Rings from the existing slide
ambience set) layered above. All four knobs are refs.

## Layer 2 — presets: the 6th token source

`tokens/presets.json` → `presets_gen.rs`, exactly like moods/fonts:

```json
{ "look":   [ { "id": "neon-intro", "font": "orbitron",
                "background": { "kind": "mood-blend",
                  "moods": ["ember-drift", "deep-lagoon"],
                  "motion": "blend-drift", "ambience": "pulse" } } ],
  "motion": [ { "id": "calm-enter", "enter": "rise", "stagger": "loose" } ],
  "interaction": [ ... ] }
```

A preset is a named bundle of refs — data composed FROM the generated
styling, zero new code per preset. The editor's "Save as preset" appends to
this authored file (the one sanctioned case of the app writing a token
source — it writes DATA in the authored format, never generated output).
Every preset renders on a Library wall (gallery gate).

## The UI law: layman-friendly, frictionless, touch-first (operator, 2026-07-22)

> "i want it layman friendly and frictionless not some high dev useless UI
> that is pointless and the goal of this app is to be my private use ... so
> i can pull whatever i want to make my life easier generating content,
> slides to have on my large 75\" touchscreen prop to help explain tech,
> ai, news"

Binding consequences for every W-pass:

- **The stack is invisible.** The operator never sees JSON, ids, refs, or
  the word "token." They see the THING: fonts drawn as themselves, moods as
  living gradient cards, motions playing their own preview. Tap it, it
  applies, on the real slide, instantly.
- **Touch-first geometry.** Every editor control honors `tap-min`; pickers
  are big swipeable trays, not dropdown menus; nothing hides behind
  right-click or hover-only affordances (hover is a bonus, never the only
  path — this runs on a 75" touchscreen).
- **No save ceremony.** Edits write back automatically (atomic), undo is
  one big obvious button. The file mechanics are our business, not theirs.
- **Plain words.** "Background", "Font", "Motion", "Theme" — never
  "preset id", "registry", "ref".
- **Private use.** No licensing/commercial constraint ever shapes a
  feature choice; pull whatever makes content generation easier.
- The purpose the editor serves: producing slides/content that explain
  tech, AI, and news on the 75" touchscreen prop — friendly enough that
  the explanation, not the tool, is the show.

## Layer 4 — the editor (Workbench)

- The stage renders the document via the ONE renderer.
- Hit-testing the painted blocks → selection (egui makes this natural: the
  renderer returns block rects; click resolves to the innermost).
- Contextual inspector per selection kind:
  - text block → font (the fonts wall as a picker), size (fluid ramp),
    ink, the text itself;
  - background → mood A/B pickers (moods wall), blend motion, ambience;
  - slide → template, enter preset.
- **Every edit mutates the in-memory doc and writes it back to the
  colocated `slide.json`** — atomic (temp + rename), pretty-printed, LF.
  Undo = a session stack of doc snapshots (the doc is a small value).
- Seeds are git-tracked, so seed edits are diffable history; episode
  slides live in content/ under the never-clobber law.

## Why there is no IPC (the operator's question)

**IPC is not banned — it is ELIMINATED.** The line in AGENTS.md ("no IPC
boundary") describes the win, not a prohibition: the OLD app was two
programs — a JS webview frontend and a Rust backend — and every feature
paid the Tauri IPC tax (serialize → boundary → deserialize, two type
systems, the seam where bugs lived). The rewrite is ONE native process, so
what used to be IPC is now a function call or a typed enum on a tokio
channel (the fabric) — faster than any IPC can ever be, and rustc checks
both ends. Talking to OTHER processes is untouched and sanctioned: ffmpeg
runs as a subprocess over pipes (ADR-009), ComfyUI will speak HTTP
(ADR-010), a future Blender bridge would do the same. deny.toml bans
ffmpeg *crates*, not process integration.

## Pass breakdown (each green + exe-proofed; plan doc per pass)

- **W1 — dpl-doc**: the document crate — serde schema, folder load/save,
  atomic write-back, validator (unknown ref → flagged, degrades in paint);
  3–5 SEED slides authored under `seeds/slides/` reproducing the current
  template demos as documents.
- **W2 — renderer**: doc → paint through the registries; `mood_blend()`
  runtime (two mood gradients, crossfade + pan via `blend-drift` in
  motion.json) + ambience layering; the Library templates wall gains a
  "seeds" section rendering every seed live (gallery gate).
- **W3 — presets**: tokens/presets.json → generated registry → presets
  wall; docs for the append-on-save contract.
- **W4 — the editor**: Workbench stage + hit-test selection + contextual
  inspectors (font/mood-blend/ambience/text) + write-back + undo. The
  proof: click the seed's title → swap Orbitron in → the colocated
  slide.json diff shows exactly one changed ref.

Sequencing note: W1/W2 make the vision RENDER, W3 makes it COMPOSABLE, W4
makes it EDITABLE. Crisp-edge work (D1) and the wgpu editor ADR (R1) run
alongside; wiredFRONT (the AV editor) is untouched by all of this — the
two-editors-never-mix law holds.
