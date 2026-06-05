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
- **Win condition** — checkmate (inferred; not stated). Solver uses King-capture as terminal.
  — [open, see rules.md]
- **Repetition** — not stated; baseline draw. — [open, see rules.md]
- **Origin** — English name by Kerry Handscomb (NOST), invention credited to Ōyama Yasuharu
  (Wikipedia); an earlier secondary source said Fujio Akatsuka — **conflict**. — [open]

> The state-space *upper bound* below does not depend on the open items (it counts
> arrangements); the *reachable* count and the *solve* depend on win-condition + repetition,
> both of which have working defaults (King-capture terminal; repetition = draw).

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
