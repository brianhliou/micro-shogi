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

## The central risk: RAM-speed vs disk-random edge access

This is the dominant project risk — bigger than position count, branching, or
pricing. It decides whether the solve costs ~$4k or is infeasible. **One
sentence:** at 5×10¹⁴ positions the table can't fit in memory, so unless the
computation is engineered carefully, every one of the ~1.6×10¹⁶ value-lookups
becomes a random disk seek — and random disk is 100–1000× slower than RAM.

- **Root cause — a speed cliff.** RAM read ≈ 100 ns; random SSD read ≈
  10,000–100,000 ns. The solve is ~1.6×10¹⁶ lookups. Multiply the count by 100 ns
  vs 10,000 ns: same computation, same result, **~$4k vs ~$400k.** The bill is set
  by *where the value lives when you reach for it*, not by the arithmetic.
- **Trigger — the data doesn't fit in RAM.** 5×10¹⁴ positions × even 1 byte =
  500 TB; a node holds ~0.25–1 TB, so ~99.9% of the table is on disk at any
  instant. **Dōbutsu hid this entirely** (214M × 32 B = 7 GB fits in RAM, every
  access free, the naive solver "just worked"). That free lunch vanishes here.
- **Why naive = random seeks.** A position's neighbors have canonical keys at
  essentially random locations across the 500 TB; "look up the neighbor's value"
  is a random disk jump. 10¹⁶ random jumps = death by seek.
- **The fix (this IS the architecture) — convert random access into sequential
  streams.** Partition the space into shards small enough that one fits in a
  node's RAM; process a shard entirely in-RAM. For every edge pointing *out* of
  the shard, don't follow it — emit a message "position Y needs the value of X"
  to an outbox. Group messages by destination shard and **stream** them in bulk
  (sequential write + read); each shard consumes its inbox as a batch. You've
  replaced 10¹⁶ random seeks with a few giant sequential passes + in-RAM sorts,
  and sequential disk is ~100–1000× faster per byte. Near-RAM effective speed
  recovered.
- **The mental model.** This is the MapReduce/Pregel **shuffle** — equivalently,
  repartitioning a stream by key through Kafka (never random-access across
  partitions; produce to the destination, consume in order). In DB terms it's the
  gap between an index-nested-loop join (random I/O, dies on big data) and a
  sort-merge/hash join (sequential, scales): same result, ~100× apart, purely
  access-pattern engineering. Same reason LSM stores batch into sorted SSTables
  instead of random in-place updates.
- **Why it's a RISK, not a given.** (1) Real distributed-systems engineering — a
  purpose-built Pregel/Spark for this one graph. (2) Drops cycle the material
  graph, so the big core can't be cleanly staged; it needs an iterative fixpoint =
  *multiple* shuffle passes, and slow convergence multiplies shuffle volume and
  cost. (3) The graph's locality is unknown until measured (neighbor scatter,
  long-range drop edges, fixpoint round count) — these set how much shuffle you
  pay. (4) RAM budgeting is fiddly: value array + per-position counters +
  inbox/outbox buffers share node RAM; mis-size a shard and you spill back to
  disk-random.
- **The failure mode isn't a bug** — it's building it correctly and *discovering*
  the effective rate is 1,000–5,000 ns/edge instead of 150–500, because this graph
  forces more shuffle than hoped. That is the 10× in the cost matrix
  (`cost-model.md`), and exactly what the partial-EGTB milestone measures before
  any cluster spend.

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
- **H5 — Random-I/O death.** The dominant cost driver — see "The central risk" above. Naive
  retrograde does random reads/writes over a 500 TB+ array; the entire architecture exists to
  convert that into sequential streaming. Make-or-break.
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
