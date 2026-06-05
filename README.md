# micro-shogi

> Research project: scope and (eventually) compute the **complete strong solution** of
> **Micro Shogi** — the 4×5 drop-shogi variant that sits one rung above Dōbutsu Shōgi on
> the small-shogi ladder. Treat it like science, not blogging: every number carries its
> source, estimates are bracketed, and the early wrong guesses are not recorded here.

This repo is the sequel to [dobutsu-shogi](../dobutsu-shogi), where we reproduced Tanaka's
2009 strong solution of Dōbutsu Shōgi from the primary source and built a from-scratch Rust
tablebase. Dōbutsu is the **largest strongly-solved drop-shogi** to date. Micro Shogi is the
next candidate that is *feasible but hard* — and the interesting question is exactly **how
hard**, in positions, dollars, and engineering.

## The headline numbers (validated)

| | Dōbutsu (3×4) — *measured* | Micro Shogi (4×5) — *this repo* |
|---|---|---|
| all-arrangements upper bound | 1,567,925,964 | **3,915,109,365,634,620** (≈3.92×10¹⁵) |
| reachable positions | 246,803,167 | **~3–6×10¹⁴** (bracket) |
| canonical (symmetry-folded) | 213,993,386 | **~5×10¹⁴** |
| complete tablebase on disk | 333 MB | **~134 TB** (W/L/D) – **~1 PB** (with DTM) |
| solve compute | 75 min, 1 core, 7 GB RAM | **~150 core-years**, ~100 PB shuffle |
| solve hardware | a laptop | **~16–20 NVMe nodes, ~1–2 months** |
| est. cost to solve | ~$0 | **~$10–15k bare-metal** / ~$40–50k cloud |

The micro-shogi upper bound is **exact** — computed by a combinatorial enumerator that
reproduces Tanaka's published Dōbutsu figure (1,567,925,964) and its full by-pieces-in-hand
breakdown to the digit. See [`research/repro/`](research/repro/). The reachable count is
bracketed from that upper bound using ratios calibrated on Dōbutsu (0.157) and Minishogi
(0.077); a direct reachable-enumerator is an open task.

## Why it's interesting

- **It's the frontier.** Dōbutsu (2.5×10⁸) is solved; Minishogi 5×5 (~2.4×10¹⁸) is firmly
  unsolved (only *estimated*, never enumerated). Micro Shogi (~5×10¹⁴) sits in between — the
  next rung that retrograde analysis can plausibly reach, but only as a real distributed,
  external-memory computation.
- **Drops break the chess-tablebase recipe.** In chess, captures strictly reduce material,
  so endgame tablebases solve a clean DAG of material buckets bottom-up. Shogi drops let
  pieces re-enter play, cycling the material graph — so the standard recipe doesn't apply
  unmodified. Dōbutsu hid this (it fit in RAM, solved whole); Micro Shogi forces the issue.
- **The cost is a design variable, not a constant.** What you store per position (2 bits vs
  16), bare-metal vs cloud, and persist-all vs stream-and-discard swing the budget ~8×, ~5×,
  and ~10× respectively. The analysis is in [`research/cost-model.md`](research/cost-model.md).

## Status

**Scoping / research.** No solver yet. The validated state-space enumerator is the only code.
The cheap, high-leverage milestones come before any cluster spend:

1. **Nail the exact ruleset** — capture-flip promotion, drop restrictions, repetition.
   *(mostly done — see [`research/rules.md`](research/rules.md); only win-condition + repetition
   refinement remain, both with working defaults)*
2. **Rules engine** ✅ + **brute-force validator** — `solver/` (Rust). Engine done and tested:
   move-gen with sliders, capture-flip promotion, either-face drops, king-capture terminal; 8
   unit tests + `perft` self-consistency (`research/repro/perft.txt`). Validator (forward minimax
   on small endgames; no external oracle for Micro Shogi) *next*. *(in progress)*
3. **Calibration solve** — a smaller sibling game (fewer piece types or a smaller board) run
   end-to-end in RAM. Proves the pipeline and *calibrates the real per-edge cost*, collapsing the
   10× compute uncertainty before the go/no-go on the full run. (Drop-shogi has no clean N-piece
   tablebase — material is conserved — so the partial is a smaller *instance*, not a piece-count
   slice.) *(open)*
4. **Full strong solve** on bare-metal — only after 1–3. *(open)*

## Layout

```
research/
  rules.md           — the exact ruleset the solver is built against (milestone #1)
  findings.md        — verified facts ledger (numbers + sources)
  open-questions.md  — the backlog (ruleset, exact reachable count, calibration)
  cost-model.md      — $ analysis with 2026 cloud/bare-metal pricing
  architecture.md    — distributed strong-solve design (SCC staging, coordinator, hurdles)
  funding.md         — funding/collaboration strategy + live 2026 programs (verified)
  repro/
    statespace_upperbound.py  — the validated enumerator
    upper_bound.txt           — its committed output
```

## Primary sources to establish

- A primary/authoritative source for the **exact Micro Shogi rules** (the gating task).
- Tanaka 2009 (Dōbutsu) — the methodological anchor, in the sibling repo.
- Minishogi reachable-count estimate: *Estimating the number of reachable positions in
  Minishogi*, arXiv:2409.00129.
