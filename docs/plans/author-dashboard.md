# Pass — the dashboard (M8)

Declare what you are doing *before* you reach the pieces. The operator's third
standing correction, and the only ratified screen with nothing behind it.

**Reference:** R1 — `docs/ops/references/authoring-mock-a3db4aee.html`,
`renderDashboard()`.
**POA&M:** `docs/plans/authoring-lane-poam.md` — M8, listed *waiting on
nothing* since the POA&M was written and not worked since.

## 1. Context — why this pass exists

Standing correction #3:

> **Reverse the IA** — declare intent at a dashboard first, *then* reach the
> pieces.

The authoring surface still opens straight onto the container bar and the
carousel. Everything downstream of the choice was built; the choice itself
never was. It has been the top unbuilt item on the ratified mock since the
reset.

## 2. Current state (ground-truthed in-session, `ae40892`)

**The mock's dashboard**, verbatim from R1:

- eyebrow `DevPULSE · Author`, then *"What are you working on today?"*, then
  *"Pick the lane first. Everything after it — what you see, what the buttons
  say, where the file lands — follows from this one choice."*
- **three lanes**, each with a glyph, a title, a description and a live
  footer count:

| lane | title | footer in the mock |
|---|---|---|
| content | Make something | `<N> open` |
| edit | Pick up where I left off | `last · <name>` |
| platform | Build a new piece | `<N> to build with` |

- an **"Open now"** section: every container the operator has, as cards with
  a kind pill, a name and `count unit · when`.

**What each number can honestly be:**

| footer | source | honest today? |
|---|---|---|
| `N open` | `containers()` across all kinds | **yes** — the census already exists |
| `last · <name>` | most-recently-modified container dir (`fs::metadata().modified()`) | **yes** — a real fact, not a stored preference |
| `N to build with` | `demos()` + `elements()` + `blocks()` + `cl::TRAITS` + `list_seeds()` | **yes** — every registry is already enumerated by the Library |

So all three counts are real. None needs a new store, and none may be
invented — the lane has refused three padded numbers already.

**`when` is the one thing the mock has and we do not.** The mock hardcodes
"2h ago". The honest version is the container folder's modified time, rendered
coarsely ("today", "yesterday", "last week"). A precise timestamp implies a
history we do not keep.

**Where the lanes go:**

- *Make something* → the container picker, then the existing content surface.
- *Pick up where I left off* → straight into the content surface on the most
  recently touched container.
- *Build a new piece* → **M9's platform lane, which does not exist.** The
  Library is the closest real surface — it IS "what you can build with", and
  routing there is honest. The lane says so rather than implying M9 shipped.

## 3. Design — the choice first, and every number real

**The dashboard becomes the Author subroute's first screen.** The content
surface is what you get *after* choosing, not instead of choosing. A `Screen`
enum on `AuthorSurface` (`Dashboard` | `Content`) carries it — the same
transient-state pattern the surface already uses.

**Three lanes, three live counts**, each read from the census that already
exists. No new registry, no stored state.

**"Open now" lists every container**, across kinds, with its kind pill, its
name, its count in that kind's unit, and a coarse "when". Tapping one is the
fastest path in the product: it *is* the choose-and-go gesture.

**Going back is a first-class control.** The container bar gains the `← Back`
deferred in `author-filmstrip.md` §4 — it belongs to this pass because until
now there was nowhere to go back *to*.

**"Build a new piece" is honest about M9.** It routes to the Library and its
footer says what that is. When the platform lane lands it re-points, and the
POA&M row records that.

## 4. Scope

**In:** the `Screen` split · the three lanes with live counts · the "Open now"
row with coarse `when` · tapping a container to go straight in · `← Back` ·
the honest routing for the platform lane · tests · proof.

**Out, and named:**
- **The platform lane itself** (M9). This pass routes to the Library and says
  so; it does not build the ingredient composer.
- **"Add a new kind"** (the mock's second button). Lesson and deck have no
  storage — a button that creates a kind nothing can hold is the padding this
  lane refuses.
- **A recents history.** `when` is the folder's modified time, not a log.
  Anything richer needs a store this pass will not invent.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `Screen` on `AuthorSurface`, defaulting to `Dashboard`; `ui()` dispatches.
   The content surface moves into its own method unchanged.
2. The three lanes with their live counts, from the existing censuses.
3. "Open now" over `containers()` across kinds, with the coarse `when`.
4. `← Back`, and the drills (`--author-container`, `--card-tab`) entering the
   content screen so existing proofs keep working.
5. `cargo green`, proof at both sizes, §8, POA&M row, `docs/STATE.md`.

## 6. Risks

- **Every existing drill and proof enters via the content surface.**
  `--author-container`, `--author-draft`, `--card-tab` and `--author-editor`
  must all still land where they did. Mitigation: those seams set
  `Screen::Content` explicitly, and the existing proofs are re-taken as the
  check.
- **A count that lies.** Mitigation: all three come from censuses that already
  have tests; a test asserts the lane counts equal the census lengths.
- **"Last touched" being empty on a fresh machine.** Mitigation: the lane says
  there is nothing to go back to, rather than showing an empty name.

## 7. Verification / acceptance

- `cargo green` clean.
- A test that each lane's count equals the census it claims to summarise.
- A test that the drills still enter the content screen.
- Real-exe proof at 1280×800 and 1100×700 of the dashboard, plus a re-taken
  content-surface shot proving the drills still work.

## 8. Evidence

### 8.1 The gate

`cargo green ✔`. **47 tests**, 3 new.

`every_drill_enters_the_content_screen` is the important one — every harness
seam (`--author-container`, `--card-tab`, `--author-draft`, `--author-editor`)
must still land where it always did, because every committed proof shot in
this lane depends on it. It also asserts the DEFAULT is still the dashboard,
which is the point of the pass.

`the_build_with_count_is_the_registries` pins the third lane's number to the
registries the Library shows — a lane quoting a number nobody can reach would
be the padding this lane keeps refusing.

`when_is_coarse_and_never_a_timestamp` pins the wording.

### 8.2 Real-exe proof

`docs/proof/author-dashboard-1280x800.png` and `-1100x700.png`, against a
scratch root seeded with one real episode:

- **Make something** — `1 open`
- **Pick up where I left off** — `last · Effingai · How AI actually works`
- **Build a new piece** — `42 to build with`
- **Open now** — `Episode · Effingai · How AI actually works · 4 cards · in
  the last hour`

Every number read from a live census. None stored, none invented.

### 8.3 Findings

**The feel bar caught a real regression, and the vocabulary already had the
fix.** The first shot was correct and barely readable: the content surface
fills its panel and hides the backdrop by sheer volume, but the dashboard is
deliberately sparse, so the artwork came straight through and the lane footers
were nearly invisible. A legibility scrim under the screen fixes it — the same
diffusion `media_block_backdrop` lays under a picture before words sit on it.
Worth recording because "it works" and "it reads" are different acceptance
tests, and only the shot catches the second.

**"When" is deliberately coarse.** The folder's mtime is the only fact
available. "in the last hour / today / yesterday / this week / a while ago" is
what that fact supports; a precise timestamp would imply a history this
surface does not keep.

**The third lane is honest about M9.** It routes to what exists rather than to
a platform lane that does not, and its tooltip says what it is.
