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
| working W/L/D table | 333 MB | **~1 PB** (arrangement-rank basis; reachable floor ~134 TB) |
| solve compute | 75 min, 1 core, 7 GB RAM | **~660–1,200 core-years**, arrangement basis |
| solve hardware | a laptop | **tens of NVMe nodes, ~1–2 months** |
| est. cost to solve | ~$0 | **~$40–70k bare-metal** / ~$150–280k cloud |

The micro-shogi upper bound is **exact** — computed by a combinatorial enumerator that
reproduces Tanaka's published Dōbutsu figure (1,567,925,964) and its full by-pieces-in-hand
breakdown to the digit. See [`research/repro/`](research/repro/). The reachable count is
bracketed from that upper bound using ratios calibrated on Dōbutsu (0.157) and Minishogi
(0.077); a direct reachable-enumerator is an open task. The full solver should be sized to the
arrangement-rank domain, not the reachable estimate, because a scalable dense rank spans legal
and unreachable slots.

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

**Scoping + calibration.** The rules engine, perft harness, push-based retrograde solver, KP
calibration solve, and browser rules viewer exist. The full external-memory solver does not.
The cheap, high-leverage milestones come before any cluster spend:

1. **Nail the exact ruleset** ✅ — capture-flip promotion, drop restrictions, and win condition.
   *(mostly done — see [`research/rules.md`](research/rules.md); repetition and per-piece
   primary-source verification remain)*
2. **Rules engine + viewer** ✅ — `solver/` (Rust) and `viewer/` (static browser app). Engine done
   and tested: move-gen with sliders, capture-flip promotion, either-face drops,
   king-capture terminal; unit tests + `perft` self-consistency (`research/repro/perft.txt`).
3. **Calibration solve** ✅ — KP (King+Pawn) solved in RAM: 457,993 canonical reachable positions,
   start value draw, max DTM 29, ~167 ns/edge, consistency audit + push-vs-pull cross-check.
   KPG is re-runnable but deferred as marginal; 4×4 is the recommended de-risk run.
4. **Full strong solve** on bare-metal — only after 1–3. *(open)*

## Layout

```
research/
  session-2026-06-05.md — full session record: all data, Q&A, open questions, next steps
  rules.md           — the exact ruleset the solver is built against (milestone #1)
  findings.md        — verified facts ledger (numbers + sources)
  open-questions.md  — the backlog (ruleset, exact reachable count, calibration)
  cost-model.md      — $ analysis with 2026 cloud/bare-metal pricing
  architecture.md    — distributed strong-solve design (SCC staging, coordinator, hurdles)
  funding.md         — funding/collaboration strategy + live 2026 programs (verified)
  repro/
    statespace_upperbound.py  — the validated enumerator
    upper_bound.txt           — its committed output
viewer/
  index.html        — standalone legal-move viewer (open directly in a browser)
  rules.js          — tested JavaScript port of the Micro Shogi rules
solver/
  src/lib.rs        — Rust rules engine
  src/retro.rs      — pull + push retrograde calibration solvers
```

## Primary sources to establish

- A primary/authoritative source for **per-piece Micro Shogi movement** beyond "standard shogi
  equivalents."
- Tanaka 2009 (Dōbutsu) — the methodological anchor, in the sibling repo.
- Minishogi reachable-count estimate: *Estimating the number of reachable positions in
  Minishogi*, arXiv:2409.00129.
