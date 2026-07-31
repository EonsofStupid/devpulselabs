# Journal — 2026-07-31: Coach Import Process + Lab capture

## Verdict

**CAPTURED** — research, mock rev, and Lab program locked for operator
look-approve. No Rust this pass.

## Why

Operator required cutting-edge Coach work to stay methodical: clear import
prompt, a11y essential checks, a **separate Advanced Import** when a11y is
missing (proprietary fabric compiled for lightweight screenshot-free
lessons), and a Lab/experiment process so Harvest / Teacher / DuckDB
baselines do not thrash signal design.

## Decisions (operator session)

| Decision | Ruling |
|---|---|
| Import honesty | Always prompt ETA + checks before deep work |
| a11y | Big essential gate for Standard vs Advanced |
| Advanced Import | Separate workflow; same FabricGraph for lessons |
| Screenshots | Import fuel only — never lesson hot path |
| Harvest | Parallel Advanced spike (CX-harvest-01), not blocking C2f; C2g only after journal |
| Teacher | In-app CoachCommand + actor first; Pass-2 parked |
| Lab | Experiment registry under `docs/research/coach/` |

## Artifacts

| Piece | Path |
|---|---|
| Mock rev | `docs/ops/references/coach-operator-mock-c2c.html` |
| Program | `docs/plans/coach-lab-experiment-program.md` |
| Research index | `docs/research/coach/README.md` |
| Import process | `docs/research/coach/import-process.md` |

## Research spine (abbreviated)

- gptme / Terminator: a11y-first computer use  
- ActionEngine / Murphy: offline harvest → state-machine fabric  
- OmniParser-class: structured vision as import fallthrough  
- Full-tree LLM dumps = anti-pattern; stamp ledger for DuckDB later  

## Operator next

1. **Look-approve** mock rev (stages 1–6).  
2. On approve → C2f plan against this Import Process (not CLI-only wire).  
3. Queue CX-teacher-agency-01 / CX-harvest-01 as Lab spikes after C2f starts or in parallel Advanced.

## Boundaries held

- Docs + mock only.  
- Overlay ADR untouched.  
- Copy/rev mock in place (prior LOOK-APPROVED noted in header).
