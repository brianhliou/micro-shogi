# CLAUDE.md — micro-shogi

> Research project: scope and eventually compute the **complete strong solution** of Micro
> Shogi (4×5 drop-shogi). Sequel to `../dobutsu-shogi`. Treat it like science.

## What this is

- **Is:** a rigorous scoping + (eventually) solving effort for Micro Shogi — the state-space
  math, the distributed-solve architecture, the cost model, and a from-scratch solver when we
  build one. The goal stated by the user is a **complete tablebase** (a true strong solution,
  all legal positions — clausecker-style), not merely the game's value.
- **Isn't:** a general shogi-variant survey, or a re-derivation of Dōbutsu (that's done in the
  sibling repo). Minishogi/Goro-Goro/etc. appear only as calibration points.

## Working method (the "scientist" contract — inherited from dobutsu-shogi)

1. **Primary source over secondary.** For the *rules*, a primary/authoritative source is
   ground truth; Wikipedia/blogs are leads to verify. Solving the wrong ruleset is worthless,
   so the exact rules are the first open question (see `research/open-questions.md`).
2. **Every number carries its source/derivation.** No figure enters `research/findings.md`
   without a pointer (a paper section, a measured run, or a committed script + its output).
   Mark each as *measured* / *validated* / *estimate (bracketed)* / *needs source*.
3. **Validate the combinatorics against a known answer.** The state-space enumerator earns
   trust by reproducing Tanaka's published Dōbutsu upper bound (1,567,925,964) exactly. Any
   new counting code does the same before its Micro Shogi output is believed.
4. **No oracle for Micro Shogi.** clausecker's tablebase is Dōbutsu-only. Correctness must
   come from: brute-force forward minimax on small buckets, two independent implementations,
   and full-table consistency audits. This is non-negotiable for a publishable solve.
5. **Estimates are bracketed, not guessed.** Where a number isn't computed exactly (reachable
   count, per-edge cost), give a calibrated range and name what would tighten it.

## Key facts so agents don't re-derive (all current best-known)

- **Micro Shogi all-arrangements upper bound = 3,915,109,365,634,620** (exact; `research/repro/`).
- **Reachable ≈ 3–6×10¹⁴; canonical ≈ 5×10¹⁴.** (Bracketed from the upper bound.)
- **Complete tablebase: ~134 TB (W/L/D) to ~1 PB (with DTM). Solve: ~150 core-years
  (range 40–475), ~100 PB cumulative shuffle.**
- **Dōbutsu benchmark (the anchor): 213,993,386 canonical positions, ~75 min, ~7 GB RAM,
  single-thread** (our Rust solver). Compact table 333 MB. This is what we scale *from*.
- **The dominant cost is compute-hours, not I/O or storage** — provided shuffle stays
  node-local (bare-metal) or same-AZ over private IPs (AWS, free). **Egress is the trap.**
- **Bare-metal beats cloud ~4× for this workload** (no spot interruption, NVMe flat-rate,
  zero egress). See `research/cost-model.md`.

## Conventions

- **Numbers:** write them exactly (e.g. 3,915,109,365,634,620). Distinguish **plies** (single
  moves) from full moves, and — critically — **state-space complexity** (reachable positions)
  from **game-tree complexity** (b^d leaf nodes). Conflating the two is the canonical error
  this project family exists to avoid.
- **Pieces (Micro Shogi):** King, Gold (↔Rook), Silver (↔Lance), Bishop (↔Tokin),
  Pawn (↔Knight). Promotion is by **capture-flip** (mandatory, no promotion zone) — *verify
  against a primary source before relying on it.*
- **Shogi terms:** 先手 = sente = first player; 後手 = gote = second player.

## Relationship to dobutsu-shogi

`../dobutsu-shogi` holds the methodological anchor (Tanaka reproduction, the Rust solver, the
state-space-vs-arrangements disambiguation). When scaling estimates, cite the *measured*
Dōbutsu numbers from there, not re-derivations. The state-space enumerator originated there
and is copied here because Micro Shogi is now its primary subject.

- Drafting publishable prose (article/blog/README hero) → invoke the `draft-voice` skill first.
  Internal ledgers (`research/*.md`) are technical reference, written directly.
