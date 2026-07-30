# Pass — putting a card into its container (M7)

The filmstrip says *"N started here, not put in yet"*. This pass makes "put it
in" a thing the operator can do.

**Reference:** R1 — `docs/ops/references/authoring-mock-a3db4aee.html`.
**POA&M:** `docs/plans/authoring-lane-poam.md` — M7, listed as *waiting on
nothing* since the filmstrip landed and not worked since.

## 1. Context — why this pass exists

The filmstrip has been telling the operator, accurately, that their cards are
**not** in the thing they chose to build into. That was the honest wording at
the time and it is now an outstanding promise sitting in the product: the band
names a gap it offers no way to close.

Every other piece is already built. The container bar knows what you are
building into. The strip knows what you have started. `dpl_doc::place_slide`
does the mutation and Studio already calls it. Only the gesture is missing.

## 2. Current state (ground-truthed in-session, `b28cabe`)

**`place_slide` is complete and careful** (`dpl_doc`):

```rust
pub fn place_slide(
    content_root: &Path, slide_dir: &Path,
    show: &str, episode: &str, chapter_id: &str, slot: Option<usize>,
) -> Result<PlacedSlide, String>
```

- **copies**, never moves — the draft stays in `inbox/`;
- **refuses an unfinished slide**: `validate` must be clean first;
- **refuses to create an episode** — placing only ever adds to one that exists;
- **rolls back** on an invalid episode, restoring the prior `episode.json`;
- **never clobbers**: `mint_slide_name` numbers a colliding folder.

So the risky part is already written, tested, and used elsewhere. This pass
adds a button and honest reporting.

**A chapter is required.** `place_slide` takes `chapter_id`, and an episode
carries `Vec<Chapter>`. `Container` currently carries `slug` ("show/episode"),
`label`, `count` and `dir` — no chapters. It needs them, or the tab needs to
load the episode when placing.

**Only Episodes can receive a card.** `place_slide` is episode-only. Of the
four container kinds: **episode** takes cards; **guide** is a different content
model (`guide.json`, steps, not slides); **lesson** and **deck** have no
storage at all (`ContainerKind::not_stored_yet`). So the gesture exists for one
kind and must say why for the rest — the same honest-empty move already used
twice in this lane.

**A minted draft may not validate.** `mint_draft` writes a schema-valid
document, but the operator may have emptied a field since. `place_slide`
refuses it, and the reason has to reach the strip in plain words rather than
being swallowed.

## 3. Design — one gesture, honest about what it did

**The gesture lives on the strip**, where the promise is: a "Put in" control
beside the count, acting on the cards started this sitting.

**Placing writes and then tells the truth.** On success the draft is removed
from the sitting's list — it is *in* the container now — and the container's
count is re-read from disk rather than incremented, so the number shown is the
number on disk. On failure the card stays in the strip and the reason is shown
verbatim from `place_slide` ("slide is not finished…", "chapter … not in
episode"), because a swallowed error here means an operator believing their
work is filed when it is not.

**The chapter is chosen when there is a choice.** One chapter, no question —
it is placed there. More than one, the strip asks which, with the chapter
titles as written. This is the only new decision the pass introduces.

**Kinds that cannot receive a card say so.** Guide, lesson and deck get the
same sentence pattern `not_stored_yet` established: what would have to be true,
in the operator's words. Guides are not a shortfall — they are a different
content model, and the wording should not imply they are broken.

**Nothing moves the source.** The draft is copied. `/content` sources are never
mutated and the inbox copy remains, which is also what makes a failed place
harmless.

## 4. Scope

**In:** the "Put in" gesture on the strip · chapter choice when there is more
than one · re-reading the count from disk after a place · verbatim failure
reporting · the honest line for kinds that cannot receive · tests · proof.

**Out, and named:**
- **Placing into a guide.** A guide holds steps, not slide folders. Making a
  card into a guide step is a content-model question, not a button.
- **Choosing the slot.** `place_slide` takes `slot: Option<usize>`; this pass
  appends. Ordering a chapter is the episode wall's job, not the strip's.
- **Creating an episode from here.** `place_slide` refuses on purpose, and the
  container bar is where a container gets chosen.
- **Removing the draft from `inbox/`.** It is a copy; the inbox is the
  operator's own scratch and deleting from it is not this pass's call.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `Container` carries its chapters (`Vec<(String, String)>`) for episodes;
   the census fills them. Tests.
2. `AuthorSurface::place_draft(ix)` — calls `place_slide`, re-reads the count,
   drops the draft on success, records the message either way.
3. The strip's "Put in" control, and the chapter choice when there is one.
4. The honest line for guide / lesson / deck.
5. `cargo green`, proof at both sizes, §8, POA&M row, `docs/STATE.md`.

## 6. Risks

- **Placing into the wrong chapter silently.** Mitigation: no default when
  there is a choice — the operator picks, and the chapter title is shown.
- **The count drifting from disk.** Mitigation: re-read, never increment.
- **A failure that reads as success.** Mitigation: the strip's line is driven
  by the `Result`, and a test asserts the draft SURVIVES a refused place.
- **Proof needs a real episode with a real chapter.** `--clone-seed` provides
  one; the drill already uses it.

## 7. Verification / acceptance

- `cargo green` clean.
- A test that placing copies into the episode, leaves the inbox draft alone,
  and grows the chapter by one.
- A test that an unfinished slide is REFUSED and the draft survives.
- A test that the count after placing equals what is on disk.
- Real-exe proof at 1280×800 and 1100×700: a card placed, the strip's count
  moved, and the honest line for a kind that cannot receive.

## 8. Evidence

### 8.1 The gate

`cargo green ✔`. **44 tests**, 6 new.

Over `place_slide` itself: `placing_copies_into_the_chapter_and_leaves_the_draft_alone`
· `an_unfinished_card_is_refused_and_survives` ·
`the_count_after_placing_is_what_is_on_disk` ·
`only_an_episode_takes_a_card_and_the_rest_explain` (which also asserts the
guide wording contains no "yet" — a guide is a different content model, not a
shortfall).

Over the surface's own gesture:
`the_surfaces_own_gesture_files_the_card_and_updates_itself` and
`a_refused_place_keeps_the_card_and_explains`. The second is the important
one: the failure this gesture must never have is silent success.

### 8.2 Real-exe proof

`docs/proof/author-place-1280x800.png` — the strip reads

> `EPISODE  Effingai · How AI actually works · already has 4 cards ·
>  2 started here, not put in yet · Put them in  The setup · What it costs ·
>  Where the news comes from`

Three chapters, offered **by their authored titles**, read off the episode the
container bar is pointed at. That is the "more than one is a decision" path,
against a real seeded episode.

### 8.3 Findings

**A global made the wiring untestable, so the global went.** `place_draft`
first called `dpl_engine_capture::content_root()`, which is a process-wide
`OnceLock` — un-pointable from a test, because the first caller in the process
wins and `cargo test` shares one. The root is now a parameter. That is better
design independently (injection over ambient state) and it is the only reason
the two surface-level tests above can exist. Recorded because "it cannot be
tested" was, on inspection, "it reads a global it did not need to".

**The risky part was already written.** `place_slide` copies rather than
moves, refuses an unfinished slide, refuses to create an episode, never
clobbers a name, and rolls the episode back if the result would be invalid.
This pass added a gesture over a safe primitive — the correct amount of new
mutation code for M7 was zero.

**The count is re-read, never incremented.** It is a fact about a folder, not
a tally the surface keeps. A test pins it.

**Placing many stops at the first refusal.** The loop files oldest-first and
breaks when a card is not accepted, rather than retrying it forever; the
reason is already in the status line and the remaining cards stay put.
