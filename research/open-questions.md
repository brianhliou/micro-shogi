# Open questions — the backlog

Resolve with primary sources or our own reproducible experiments. Don't paper over in prose.

## Gating (blocks everything downstream)

- [ ] **Exact Micro Shogi ruleset from a primary/authoritative source.** Specifically:
  - The precise capture-flip promotion cycle and whether promotion is *only* on capture.
  - Drop restrictions: is there a two-pawn (nifu) rule? A drop-pawn-mate (uchifuzume) ban?
    Any restriction on dropping a piece where it would have no legal move (e.g. pawn/knight on
    the last rank)?
  - Repetition (sennichite): draw, or loss for the perpetual-checking side? This changes the
    fixpoint (perpetual-check asymmetry is harder than a symmetric draw rule).
  - Whether an unpromoted forward-only piece may legally occupy the last rank (no promotion
    zone means the standard "must promote" rule may not apply).
  - **Why it gates:** solving the wrong ruleset produces a worthless tablebase. The value of
    the game and the graph structure both depend on these.

## Tighten the estimates

- [ ] **Exact reachable count** (replaces the 0.077–0.157 bracket → a single number). Requires
  a correct rules engine + a canonical-key + a BFS/enumerator. Folds turn+LR symmetry. This
  also produces the exact target count that the solve must reproduce (completeness check).
- [ ] **Per-edge cost and fixpoint pass-count calibration.** The compute estimate carries ~10×
  uncertainty (40–475 core-years). The partial-EGTB milestone measures real ns/edge and the
  number of passes on actual hardware, collapsing the range before the cluster go/no-go.
- [ ] **Average branching factor for Micro Shogi.** Estimated ~16 (Dōbutsu measured 9.435);
  drives the ~100 PB shuffle figure. Measure on the rules engine.
- [ ] **Max distance-to-mate (depth).** Sets DTM bit-width and the number of fixpoint
  rounds/supersteps. Dōbutsu was 173; Micro is presumably deeper. Unknown until partially
  solved.
- [ ] **Strength of symmetry folding.** Our Dōbutsu solver captures only ~1.15× over Tanaka's
  reachable, vs a theoretical ~4× (turn×LR). How much is actually achievable for Micro, and
  what does it save in storage/compute?

## Architecture / correctness

- [ ] **Material-signature SCC structure.** Enumerate Micro Shogi material signatures, build
  the capture/drop dependency graph, condense to SCCs, and measure the mass of the largest
  SCC (the irreducible external-memory core). Confirms whether stream-and-discard meaningfully
  bounds peak storage.
- [ ] **Can shuffle be kept node-local?** The bare-metal cost advantage (~4×) depends on the
  SCC-staging keeping most shuffle within a box's NVMe rather than across the network
  (Hetzner caps at 10 Gbps/node). If not, AWS single-AZ (100 Gbps, free same-AZ) wins.
- [ ] **Validation strategy with no oracle.** Concretely: which small material buckets get a
  brute-force forward-minimax cross-check; what the full-table consistency audit asserts
  (e.g. every won position has a child that is a loss-at-DTM−1); whether a second independent
  engine is worth building.

## Decisions pending from the user

- [ ] **W/L/D-only vs full DTM** as the persisted artifact (~8× cost swing; optimal play is
  recoverable from W/L/D via probe-time local search).
- [ ] **Bare-metal vs cloud** default (~4× cost; bare-metal accepts the node-local-shuffle
  constraint).
- [ ] **Is the goal the complete tablebase, or would a weak solve** (game value + a strategy
  from the start, via df-pn / proof-number search over a partial EGTB) **suffice** as the
  publishable result at a fraction of the cost? (User has said: complete tablebase.)
