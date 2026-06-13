# Solver method tradeoffs

These notes explain the solver choices that are easy to miss when reading only
the run commands. The short version: the current Micro Shogi calibration driver
is a convenient in-RAM shortcut, while Shogi4 already has the production-shaped
method.

## The indexing choice

Every retrograde solver needs a way to answer: "what slot stores this position's
value?"

### Reachable HashMap index

The current Micro Shogi calibration solver enumerates reachable positions from a
start position, stores each canonical `u128` key in a `Vec`, and stores
`key -> dense id` in a `HashMap`.

Pros:

- Simple and fast to write.
- Stores only reachable positions, not impossible arrangement holes.
- Good for small calibration rungs such as KP.

Cons:

- The `HashMap<u128, u32>` is memory-heavy and cache-missy at large scale.
- It cannot support a full external-memory solve: building a reachable-only
  minimal perfect hash for ~10^14-10^15 keys would itself require holding the
  keys.
- Its apparent small-game speed can be misleading because it measures a
  RAM-resident convenience path, not the scalable path.

### Dense rank/unrank index

The Shogi4 solver uses a combinatorial ranker: `rank(position) -> integer` and
`unrank(integer) -> position`. The value table is a flat array indexed by that
integer.

Pros:

- No giant reachable-key `HashMap`.
- Flat arrays can be sharded, memory-mapped, checkpointed, and streamed.
- This is the scalable tablebase pattern used by chess EGTBs and by the Shogi4
  production design.

Cons:

- The rank domain is all arrangements in the bucket, not just reachable
  positions. That means storing and scanning holes.
- Rank/unrank adds per-edge CPU cost.
- The ranker is game- and bucket-specific engineering that must be validated
  carefully.

The key lesson from Shogi4 applies here: "reachable positions" can be the
published/compressed artifact size, but a scalable solver usually pays for an
arrangement-domain index while it is computing.

## The predecessor choice

A push retrograde solver works backward from resolved positions. When child `c`
gets a value, every predecessor `p` with a legal move `p -> c` may become
resolved.

There are two ways to get those predecessors.

### Store the reverse graph

The current Micro Shogi `solve_push` builds a CSR predecessor graph:

- `par_off[child]..par_off[child + 1]` gives the slice of predecessor ids.
- `par_idx` stores one `u32` predecessor id per edge.

Pros:

- Avoids writing an unmove generator.
- Propagation is simple and mechanically checkable.
- Good for a one-off in-RAM calibration if the graph fits.

Cons:

- Memory is `O(positions + edges)`, not just `O(positions)`.
- High-branching drop games can have billions of reverse edges even when the
  position count sounds moderate.
- It does not scale to KPGS or the full game.

The 2026-06-11 KPG run exposed the hidden cost. The predecessor offsets were
stored as `u32`, and the run panicked after wraparound:

```text
wrapped par_idx length = 1,241,100,710
actual edges if one u32 wrap = 5,536,068,006
par_idx alone = 20.62 GiB at 4 bytes/edge
```

The corrected rerun completed, but it confirmed the diagnosis: KPG has
869,287,068 canonical reachable positions, used ~60.5 GiB peak RSS on a 64 GB
box, and processed 4,567,032,875 resolved-child propagation edges. The 5.54B
number is the stored predecessor capacity exposed by the overflow; propagation
touches fewer edges because draw children are never popped from the resolved
queue. That is before counting table dump space or audit time. A 64 GB box is
therefore marginal for the current KPG implementation, not comfortable.

### Generate predecessors on demand

The Shogi4 push solver does not store the reverse graph. It implements
`predecessors(child)`, ranks each predecessor, and updates its counter/value.

Pros:

- Memory is closer to `O(rank-domain positions)`: value array, child counters,
  legality mask, queue/inbox.
- It avoids storing one predecessor id per edge.
- It is the method that can be sharded: each resolved child emits predecessor
  updates to the owning rank-range shard.

Cons:

- The unmove generator is the hard correctness primitive.
- Each edge costs more CPU because predecessor generation plus rank calculation
  happens during propagation.
- The implementation must account for every rule edge case.

For Micro Shogi, `predecessors(child)` has to invert:

- board moves by steppers and sliders,
- capture-flip promotion,
- drops on either face,
- captures that put the captured base kind into hand,
- turn normalization and left-right canonicalization.

A safe implementation should generate candidates and verify them by applying the
normal forward `make()` path and checking that the canonical child is the target.
That is slower, but it keeps correctness tied to the already-tested forward
engine.

The KPG dense-rank comparison run on 2026-06-12/13 confirms the memory tradeoff
in practice. The raw rank domain is 2,037,557,340 slots, 2.34x the 869,287,068
reachable positions, but the dense solver reproduced the audited CSR result with
~6.86 GiB peak RSS instead of ~60.55 GiB. After mirror-aware predecessor
generation, single-thread wall time was 8,464.0 s, 1.46x faster than the
stored-CSR core solve and 1.85x faster than the CSR article run's full wall time.

The first dense KPG run found the mirror bug plainly: it generated 16.3B
predecessor candidates, kept 8.16B canonical predecessor ids, and discarded
another 8.16B duplicate ids. The mirror-aware rerun generated 8.16B candidates,
kept 8.16B ids, and discarded none.

## Can Micro Shogi switch to the Shogi4 method?

Yes. It is the right direction for the full solve and probably for any serious
4x5 partial tablebase. It is not a small patch to the current KPG driver.

Required work:

- Build and validate a Micro Shogi dense rank/unrank index for material buckets
  or SCC buckets.
- Implement and validate Micro Shogi unmove generation.
- Rework the push solver to store flat arrays indexed by rank instead of
  reachable `HashMap` ids.
- Add validation gates: `rank(unrank(i)) == i`, legal set equality on small
  buckets, predecessor set equality against inverted forward edges on small
  buckets, and value agreement with the existing HashMap solver where small
  enough.
- Only then add sharding/external-memory backing.

Tradeoff:

- For the current KPG article rung, the fastest path is still the existing
  reachable-HashMap solver on a large enough RAM box, or a narrowly optimized KPG
  variant with better progress/preflight reporting. It is scientifically useful
  calibration, but not the production architecture.
- For the full Micro Shogi solve, the Shogi4 method is not optional. A stored
  reverse graph and reachable HashMap are scaffolding; dense rank plus unmove
  generation is the bridge to a sharded tablebase.

## Practical rule

Use the reachable HashMap solver to answer small scientific questions quickly.
Use dense rank/unrank plus on-demand predecessors when the result is meant to
de-risk the production tablebase architecture.
