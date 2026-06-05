# Architecture — distributed strong-solve of Micro Shogi

Design notes for computing the complete tablebase (~5×10¹⁴ canonical positions). Nothing here
is built yet; this is the scoped plan. The throughline: at ~5×10¹⁴ positions the data does not
fit in RAM (≈800 TB naive) or even on one disk, so this is an **external-memory / distributed
retrograde analysis**, not the in-RAM flat-array solve that worked for Dōbutsu.

## The structural problem: drops break the chess-tablebase recipe

Chess endgame tablebases scale because captures **strictly reduce material**, so material
signatures (KQvKR, …) form a **DAG**: solve few-piece buckets first, feed them as read-only
inputs to richer buckets, never loop, each bucket fits in RAM.

**Drops destroy the DAG.** A captured piece enters the hand and can be dropped back; ownership
and board-occupancy oscillate, so the material-signature graph has **cycles**. You cannot
topologically order all buckets. This is exactly why our Dōbutsu solver solves the whole space
at once with a global fixpoint — fine at 2×10⁸, fatal at 5×10¹⁴.

**The fix:** build the material-signature graph, **condense to SCCs** (Tarjan). Most signatures
are tiny (few pieces ⇒ small placement combinatorics); the cycles cluster into a few large
SCCs. Process the **SCC-condensation DAG in topological order** (recovers chess-EGTB staging at
the coarse level); within each SCC run a distributed external-memory fixpoint (the irreducible
hard core — the largest SCC holds most of the 5×10¹⁴ mass, so staging bounds *peak* working set
but does not avoid the distributed fixpoint).

## Unit of work — two levels

- **Coarse (scheduling): a material bucket, or an SCC of buckets.** ~thousands of these. What
  the coordinator schedules against the dependency DAG. Few-piece buckets are leaves; the
  big in-play SCC is the apex.
- **Fine (data parallelism): a key-range shard within a bucket** — positions whose canonical
  key falls in `[a, b)`, sized so one shard's value array (a few hundred MB – few GB) fits in a
  worker's RAM as an mmap'd file. This is the actual parallel grain.

## What the coordinator does

- Owns the **SCC-DAG manifest**; frontier-schedules buckets — a bucket becomes *ready* when all
  its dependency buckets are `converged`. Independent ready buckets/SCCs run concurrently.
- Within an active SCC, drives **BSP supersteps**: issue a round, barrier on all shards
  reporting "decisions this round (count)", declare the SCC converged on the first **global
  zero-decision round** (the distributed lift of the single-machine `decided == 0`).
- Assigns shards to workers; on worker death, **reassigns and replays** from the shard's last
  checkpoint (a shard is a file + a deterministic recompute ⇒ exact, idempotent recovery).
- Tracks **checkpoint generations** per shard and the converged frontier for restart and
  progress reporting.

## How parallelism is achieved — push-based label propagation, BSP-style

Invert our current *pull* design (which rescans all unknowns every round). Retrograde wants to
be **push**: when position `p` gets a value, notify its **predecessors** (positions `q` with a
legal `q → p`). Predecessors land in arbitrary shards, so each superstep emits a **shuffle**
keyed by predecessor-shard — Pregel/Giraph vertex-centric, or hand-rolled over Kafka topics /
object-store partitions. Within a superstep, shards are **embarrassingly parallel**: each
consumes its inbox of "child decided → re-evaluate me" messages, recomputes only touched
positions, emits the next wave. Cross-shard dependency is realized as the shuffle *between*
supersteps, never as shared memory.

Process positions in **increasing-DTM frontier order** so each is decided once — minimizing
redundant shuffle. (The single-machine "cache adjacency" optimization noted in the Dōbutsu
solver README is the same idea; distributed, you cache backward edges implicitly via unmove-gen
+ shuffle.)

## The hard new primitive: unmove generation

Push needs `predecessors(p)`: all `q` that could legally move to `p`. For Micro Shogi this
includes **un-drop** (a board piece at `p` was just dropped ⇒ `q` had it in hand) and
**un-capture** (`p` followed a capture ⇒ `q` had an enemy piece on the destination, mover
elsewhere, un-flipped per the capture-flip rule). Un-capture *adds pieces back* and branches —
the fiddly part. Our Dōbutsu solver is **pull-based** specifically to avoid writing unmove-gen;
at 5×10¹⁴ the rescan is unaffordable, so **a correct unmove generator is the gating engineering
task** for the distributed solve.

## Incremental progress / checkpointing

- Each shard's value array is durable state on object storage; persist every K supersteps.
  Restart = reload shard files + replay inbox from the last barrier. Deterministic ⇒ exact.
- Partial results are **useful before completion**: solve all ≤k-piece buckets first and you
  already have a real endgame tablebase the (future) explorer can probe.

## Holes & hurdles

- **H1 — No external oracle.** clausecker is Dōbutsu-only; a 5×10¹⁴ table has nothing external
  to check against. Mitigations: brute-force forward minimax on small material buckets (exact,
  independent); two independent engine implementations; a full-table consistency audit (every
  win has a child that is a loss-at-DTM−1, etc.). Non-negotiable for a publishable solve.
- **H2 — Rules not nailed down.** Capture-flip details, drop restrictions, last-rank legality,
  repetition. Solving the wrong ruleset is worthless. (See `open-questions.md` — the gate.)
- **H3 — Repetition/draws.** Sennichite and perpetual-check asymmetry make the fixpoint harder
  than pure W/L; the "draw = unresolved after fixpoint" trick handles symmetric draws but not
  perpetual-check-loses rules.
- **H4 — Drops cycle the material graph.** Addressed by SCC condensation; the big SCC remains
  the irreducible external-memory core.
- **H5 — Random-I/O death.** Naive retrograde does random reads/writes over a 100 TB+ array;
  seeks dominate (100–1000× slowdown). Must restructure as sequential streaming + external
  sort/merge, or partition so each bucket's working set fits a node's RAM. Make-or-break.
- **H6 — Multi-week job will fail mid-run.** Spot reclaim, node death. Needs idempotent,
  generation-stamped checkpoints or a late failure is catastrophic.
- **H7 — Completeness proof.** Proving *all* reachable/legal positions were enumerated — a
  missed region silently corrupts values. Needs an independent reachable-enumerator to produce
  the exact target count to audit against.

## Optimizations / levers

1. **W/L/D first, DTM second (or never).** 2-bit pass is ~8× cheaper than DTM; fill DTM only
   where needed or recompute at probe time. Biggest single storage lever (1 PB → 134 TB).
2. **Stream-and-discard via SCC staging.** Peak storage ~10–30 TB instead of ~1 PB.
3. **Frontier BFS by DTM layer**, not Jacobi rescans — each position decided once.
4. **Stronger symmetry folding** (turn × LR up to ~4×; our solver currently captures ~1.15×).
5. **Compression** — EGTBs compress 4–8× (clausecker hit ~1 bit/pos on Dōbutsu).
6. **Same-AZ / node-local shuffle** — dodges AWS inter-AZ ($0.02/GB) and egress entirely; free
   on bare-metal. This is also the constraint that determines bare-metal viability (see
   `cost-model.md`).

## Milestone ordering (de-risk before spend)

1. Exact ruleset from a primary source *(gates all)*.
2. Rules engine + brute-force forward-minimax validator *(≈free)*.
3. Partial EGTB — few-piece buckets, GB-scale — proves the pipeline and **calibrates per-edge
   cost / pass count**, collapsing the 10× compute uncertainty *(≈free)*.
4. Full strong solve on bare-metal — only after 1–3, with a tightened estimate.
