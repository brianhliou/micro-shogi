# Findings ledger — verified facts

Every entry is tagged: **[measured]** (a run we did), **[validated]** (computed by code that
reproduces a known published answer), **[estimate]** (bracketed, with calibration named), or
**[needs source]** (believed from secondary sources, primary confirmation pending).

---

## The game

Full ruleset in `rules.md` (source: Wikipedia *Micro shogi*). Summary:

- **Board** — 4×5 = **20 squares**. — [Wikipedia]
- **Pieces, 5 per side** — King, Gold, Silver, Bishop, Pawn (total 10). — [Wikipedia]
- **Setup** — each player: nearest rank `S G B K` (King in right corner) + a Pawn in front of
  the King. — [Wikipedia]
- **Promotion is by capture-flip** — no promotion zone; a piece flips to its reverse when it
  captures (mandatory), and flips back when the promoted piece captures. Reverses:
  King↔(blank), Gold↔Rook, Silver↔Lance, Bishop↔Tokin, Pawn↔Knight. — [Wikipedia]
- **Drops are unrestricted** — captured pieces enter the hand and re-drop as in shogi, but with
  **no nifu, no uchifuzume, no last-rank ban**, and may be dropped with **either face up**.
  Trapped/no-move pieces are legal. — [Wikipedia]
- **Win condition** — **checkmate** ("相手の玉将を詰めたほうが勝ち"); solver uses King-capture
  terminal. — [JA Wikipedia, confirmed]
- **Repetition** — undocumented in both sources; baseline draw. — [open, see rules.md]
- **Origin** — invention credited to **Ōyama Yasuharu** (both EN + JA Wikipedia); existed by
  1982; English name by Kerry Handscomb (NOST). The earlier "Akatsuka" claim was erroneous.
  — [2 sources]

> Ruleset corroborated by two independent sources (EN + JA Wikipedia); see `rules.md`. The
> state-space *upper bound* below does not depend on any rule detail (it counts arrangements);
> the *reachable* count and *solve* depend on the (now-confirmed) win condition and the
> repetition default (draw).

---

## State-space — the validated count

Computed by `repro/statespace_upperbound.py`, which reproduces Tanaka 2009's published
Dōbutsu upper bound **1,567,925,964** and its full by-pieces-in-hand breakdown exactly, then
applies the identical model to Micro Shogi.

- **All-arrangements upper bound (legality- and reachability-ignoring)** —
  **3,915,109,365,634,620** (≈ 3.92×10¹⁵). — [validated]
  - By pieces in hand: 0→2,746,132,778,188,800; 1→998,593,737,523,200;
    2→156,030,271,488,000; 3→13,602,639,052,800; 4→725,140,684,800; 5→24,290,426,880;
    6→502,329,600; 7→5,909,760; 8→30,780.
  - Model: Kings = one per player, always on board (capturing a king ends the game). Each of
    the 4 other types has 2 copies, freely distributed by owner, board-or-hand. On-board state
    per occupied square = owner(2) × face(2, all four types flip). In-hand = base only,
    (h+1) owner-splits for h in hand. This is the Tanaka Table-1 method, verified on Dōbutsu.

- **Reachable from the start** — **~3.0×10¹⁴ … 6.2×10¹⁴** (point estimate ~5×10¹⁴). — [estimate]
  - Bracket = upper bound × (reachable/upper ratio). Calibration: Dōbutsu ratio =
    246,803,167 / 1,567,925,964 = **0.1574**; Minishogi ratio = 2.38×10¹⁸ / 3.10×10¹⁹ =
    **0.0768**. The reachable fraction *shrinks* as boards grow, and Micro (20 sq) lies
    between Dōbutsu (12) and Minishogi (25), so its true ratio is expected inside this bracket.
  - A direct reachable-enumerator (BFS over a correct rules engine) would replace the bracket
    with an exact number — see `open-questions.md`.

- **Canonical (symmetry-folded, what a solver stores)** — **~5×10¹⁴**. — [estimate]
  - Using our Dōbutsu solver's empirical fold ratio canonical/reachable = 213,993,386 /
    246,803,167 = **0.8671**. Full turn×LR symmetry could fold up to ~4×; our current solver
    captures only ~1.15× over Tanaka's reachable, so stronger folding is a real (bounded)
    storage lever.

### Calibration anchors

| Game | squares | upper bound | reachable | ratio |
|---|---|---|---|---|
| Dōbutsu (3×4) | 12 | 1,567,925,964 [validated] | 246,803,167 (Tanaka) | 0.1574 |
| **Micro (4×5)** | 20 | **3.92×10¹⁵ [validated]** | **~5×10¹⁴ [est]** | ~0.08–0.16 |
| Minishogi (5×5) | 25 | 3.10×10¹⁹ [validated] | 2.38×10¹⁸ (arXiv est) | 0.0768 |

---

## The Dōbutsu benchmark (the anchor we scale from)

All [measured] on our from-scratch Rust solver (`../dobutsu-shogi/solver`), validated
position-by-position against clausecker/dobutsu.

- **213,993,386** canonical reachable positions (turn + LR-mirror folded).
- Retrograde fixpoint: **~75 min, ~7 GB RAM, single-threaded**, fully in-RAM.
- Initial position = **−78** (gote wins in 78 plies); max distance-to-mate **173 plies**.
- Compact tablebase: **333 MB** on disk (minimal perfect hash + 9-bit DTM, ~400 MB resident).
- Algorithm: BFS-enumerate → pull-based Jacobi fixpoint (each round, regenerate forward moves,
  read children's prior-round values, decide W/L/unknown; leftovers after convergence = draws).
- No-drops ablation: **797,658** positions (~270× smaller), initial value **draw**, max DTM
  37 — direct evidence the drop rule is what makes these games deep.

---

## Calibration ladder (first solved values + measured ns/edge)

In-RAM retrograde solve of reduced-piece sub-games (real 4×5 board/engine, fewer piece
types) — `solver/`, `cargo run --release --bin solve`. Validated by a full-table
**consistency audit** (every value = minimax of its children ⇒ a distance-consistent
labeling, provably correct given correct base cases). [measured]

| Rung | pieces/side | canonical reachable | start value | max DTM | avg branching | ns/edge | audit |
|---|---|---|---|---|---|---|---|
| KP | K, P | 457,993 | **draw** | 29 | 6.86 | **167** | PASS |
| KPG | +Gold | _(running)_ | | | | | |
| KPGS | +Silver | _(pending)_ | | | | | |

- **ns/edge ≈ 167 confirms the cost model's ~150 ns RAM-speed floor** — the key
  calibration result. [measured]
- First solved fact: **King+Pawn Micro Shogi is a draw** (insufficient mating material),
  max DTM 29. (Even here, 135,804 positions are wins — king-trapping / knight forks.)
- Branching is **rung-dependent**: KP is sparse (6.86); the full game runs higher (perft
  early-game 9→12.5 and climbing; mid-game with hands ≥16). The full-game branching is the
  cost-model input, not KP's.

> Validation note: an independent *forward-minimax* cross-check proved intractable at this
> scale — the game is drawish + cyclic, so forward search is exponential and deep (it
> overflowed the stack). The rigorous validation is the **consistency audit** (a
> distance-consistent minimax fixpoint is correct) plus a **push-vs-pull cross-check**: two
> independent retrograde algorithms (Jacobi `solve` and counter-BFS `solve_push`) produce
> **identical** values on KP (0 mismatches). A code-implementation diff (Fairy-Stockfish /
> hachu) remains the pre-publication move-gen check (`open-questions.md`).

> **Pull-based Jacobi is non-viable at scale on drop-shogi.** It rescans every undecided
> position each round; draws are *never* decided, so ~70% of positions are rescanned for
> ~max-DTM rounds → `O(draws × branching × maxDTM)`. Dōbutsu survived this (1.2% draws);
> KPG (≈70% draws) ran **>8 h without converging** and was killed. The **push-based**
> counter-BFS touches each edge once and never revisits a draw — KP: 10.8 s → **1.15 s**
> (~9×), and the gain grows with max-DTM. The full solve must be push-based (and streaming).

## Sizing the complete Micro Shogi tablebase

From canonical ≈ 5×10¹⁴ (see `repro/upper_bound.txt`):

- **Storage** — [estimate]
  - W/L/D only (2 bit): **~134 TB**
  - + DTM (~10 bit): **~668 TB**
  - + DTM with slack (16 bit): **~1 PB**
  - Compressed W/L/D (EGTBs compress 4–8×; clausecker hit ~1 bit/pos on Dōbutsu): **~20–40 TB**
    as a downloadable artifact.
- **Compute** — [estimate]
  - **~150 core-years** (range 40–475; the spread is per-edge cost × number of fixpoint
    passes — the dominant uncertainty, calibrated by the partial-EGTB milestone).
  - **~100 PB cumulative shuffle** (5×10¹⁴ positions × ~16 avg-branching × ~12 B/message).
  - Binding constraint is **CPU**, not I/O: a ~1-month run ⇒ ~1,800 cores ⇒ ~16–20 NVMe nodes;
    those nodes carry the I/O and storage with headroom.

### ⚠ Correction — the figures above are reachable-based; the *index domain* is larger

Carried from `../shogi4`, which hit this head-on when its dense ranking function was built and
its `N` came out to exactly 2× the arrangement upper bound.

The numbers above (134 TB, ~150 core-years) are sized to the **canonical *reachable*** count
(~5×10¹⁴). That is the footprint **only if the solver can index reachable-only**. At this scale it
cannot, and the combinatorial "canonical key" in `architecture.md` does not:

- **Reachable-only indexing needs an MPHF** over the reachable keys (Dōbutsu's 333 MB trick).
  Building one requires holding all ~5×10¹⁴ keys (~4 PB) during construction — infeasible.
  Dōbutsu's MPHF worked because 2.5×10⁸ keys fit in RAM; **it does not scale up.**
- **A combinatorial canonical key** (closed-form rank over a material bucket's placements — the
  scalable option, used by 7-piece chess EGTBs / Lomonosov) necessarily spans **all arrangements**
  of each bucket (legal + illegal + unreachable slots), not the reachable subset. Summed over
  buckets, the key space is the **LR-folded *arrangement* count ≈ 3.9×10¹⁵**, ~7.8× the folded
  *reachable* ~5×10¹⁴. So `findings` line "canonical = what a solver stores ~5×10¹⁴" folded the
  wrong base (it folded reachable, not arrangements).

| | reachable basis (as written above) | **arrangement basis (a combinatorial-rank solver)** |
|---|---|---|
| W/L/D flat array | 134 TB | **~1 PB** (LR-folded) |
| edge-ops / compute | 1.6×10¹⁶ → ~150 core-years | **~1.25×10¹⁷ → ~660–1,200 core-years** |
| cost (bare-metal) | ~$10–15k | **~$40–70k** (cloud ~$150–280k) |

The reachable figure (~5×10¹⁴ / 134 TB) stays valid as (a) the **compressed downloadable artifact**
*after* solving (post-hoc you can enumerate + pack reachable) and (b) the theoretical floor — just
not as the **working** footprint. Net: Micro is genuinely **PB-scale, ~$40–70k+**, which is exactly
why `../shogi4` (4×4, ~19× smaller arrangement domain, ~$5–15k) is the right run to de-risk the
pipeline on first. The measured ns/edge (~167) and the SCC/push architecture are unaffected — only
the *count of slots* changes. **[estimate — carried from shogi4, 2026-06-05]**

> Caveat / open: this assumes the `architecture.md` canonical key is a combinatorial rank over
> arrangements. If a feasible reachable-only index exists at 5×10¹⁴, the original figures stand —
> but none is known, and the MPHF route is ruled out by construction cost. Worth nailing down in
> `architecture.md`.

---

## State-space vs game-tree (a number-hygiene note carried from the sibling repo)

Two different axes, routinely conflated:

- **State-space complexity** = reachable positions. The axis that decides strong-solvability by
  retrograde analysis. (Dōbutsu 2.5×10⁸; Micro ~5×10¹⁴; Minishogi ~2.4×10¹⁸; chess ~10⁴⁴⁻⁴⁶;
  standard shogi ~6.6×10⁶⁸–10⁷¹; Go ~10¹⁷².)
- **Game-tree complexity** ≈ b^d. The Shannon-number axis. (Chess ~10¹²³; standard shogi
  ~10²²⁶; Go ~10³⁶⁰.) Dōbutsu's game tree is ~10⁷⁶ yet it is *solved* — because retrograde
  works on the state space, not the tree.

Any 10^x figure recorded here must say which axis it is.
