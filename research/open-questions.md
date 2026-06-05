# Open questions — the backlog

Resolve with primary sources or our own reproducible experiments. Don't paper over in prose.

## Gating — MOSTLY RESOLVED (see `rules.md`)

The ruleset is pinned in `rules.md` from the best available source (Wikipedia *Micro
shogi*). Resolved: capture-flip promotion (mandatory, reversible), drops are
**unrestricted** (no nifu / uchifuzume / last-rank ban; droppable with either face),
trapped/no-move pieces are legal. Two genuine gaps remain before the full solve:

- [ ] **Exact winning condition.** Not stated by the source. Default chosen:
  checkmate, implemented as **King capture** (equivalent for win/loss). Confirm there
  is no special terminal rule (no Dōbutsu-style "Try"/king-entry win). Low risk, but
  verify before publishing a value.
- [ ] **Repetition (sennichite) rule.** Not stated. Baseline: **repetition → draw**
  (as in the Dōbutsu solver — unresolved-after-fixpoint = draw). Decide whether to
  model the orthodox perpetual-check-loses refinement (adds fixpoint asymmetry, H3)
  before the full solve.
- [ ] **Primary source for origin/attribution** (Ōyama vs Akatsuka conflict) — needed
  only for published prose, not for the solve.

## Tighten the estimates

- [ ] **Exact reachable count** (replaces the 0.077–0.157 bracket → a single number). Requires
  a correct rules engine + a canonical-key + a BFS/enumerator. Folds turn+LR symmetry. This
  also produces the exact target count that the solve must reproduce (completeness check).
- [ ] **Per-edge cost and fixpoint pass-count calibration.** The compute estimate carries ~10×
  uncertainty (40–475 core-years). The partial-EGTB milestone measures real ns/edge and the
  number of passes on actual hardware, collapsing the range before the cluster go/no-go.
- [~] **Average branching factor.** Measured early-game via `perft` (`repro/perft.txt`):
  9 → 12.5 by depth 6 and climbing. With unrestricted either-face drops, mid/late-game branching
  is likely **≥16** (each hand piece → ~empty-squares × 2 faces on a 20-square board), so the
  cost model's ~16 may be *low* — nudging the ~100 PB shuffle / ~150 core-year figures up. Full
  reachable-set average pending the solver.
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
