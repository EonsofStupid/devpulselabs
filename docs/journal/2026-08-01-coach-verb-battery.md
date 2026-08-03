# Journal — 2026-08-01: methodical shipped-verb battery

## Verdict

**GREEN** — all shipped catalog verbs exercised live on Calculator via
`dpl-coach-daemon` (`127.0.0.1:7843`). Full transcript:
`docs/proof/coach-verb-battery/REPORT.md`.

## Matrix

| Surface | Result |
|---|---|
| health / verbs / inventory | PASS |
| presence unreal→real / wait_until / assert_state | PASS |
| open / close | PASS |
| geom / move / resize / focus | PASS |
| observe SSE (`moved`/`resized`) | PASS |
| type_text / press_key / assert display | PASS |
| select | PASS |
| scroll | SOFT — ok via xdotool_wheel, `effect_observed:false` |
| import / resolve / invoke(click) / mount | PASS |
| drag (planned) | PASS — correctly refused |

## Counts

25 PASS · 1 SOFT · 0 FAIL

## Next

Teacher client · Cursor on the verb plane · `drag` P2.
