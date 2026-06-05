# Micro Shogi — ruleset (the solver's spec)

> Milestone #1, the gate. The solver is built against THIS document. Tags:
> **[confirmed]** (stated by the best available source, Wikipedia *Micro shogi*),
> **[standard]** (Wikipedia says pieces "move like their shogi equivalents"; the
> movement is the standard shogi definition for that piece name), **[open]**
> (under-documented; a default is chosen and flagged for verification).

**Best available source:** Wikipedia, *Micro shogi* —
<https://en.wikipedia.org/wiki/Micro_shogi>. A true *primary* source (original
Japanese rules; the game is "五分摩訶将棋 / gofun maka shōgi", English name coined by
Kerry Handscomb of NOST) would be better and is still worth finding — see
`open-questions.md`.

## Board & starting position

- **Board:** 4 files × 5 ranks = 20 squares. — [confirmed]
- **Each player:** 5 pieces — King, Gold, Silver, Bishop, Pawn. — [confirmed]
- **Setup:** each player's nearest rank is `S G B K` with the **King in the right
  corner**, and a **Pawn in front of the King** (the King's file). — [confirmed]

```
        a  b  c  d
rank 5   k  b  g  s     ← Gote (second player) back rank
rank 4   p  .  .  .     ← Gote pawn (front of gote's king)
rank 3   .  .  .  .
rank 2   .  .  .  P     ← Sente pawn (front of sente's king)
rank 1   S  G  B  K     ← Sente (first player) back rank
```
(uppercase = Sente, lowercase = Gote; the position is symmetric under 180° rotation.)

## Pieces & movement

Each non-King piece has a front (base) and a reverse (promoted) side. Movement is
the standard shogi movement for each name. — [standard]

| Base | Base move | Reverse | Reverse move |
|---|---|---|---|
| **King** (K) | 1 square any of 8 directions | — (blank) | King never promotes |
| **Gold** (G) | 1 square: 6 dirs (orthogonal + forward diagonals; not back-diagonals) | **Rook** (R) | any number orthogonally |
| **Silver** (S) | 1 square: forward + 4 diagonals (5 dirs) | **Lance** (L) | any number straight forward |
| **Bishop** (B) | any number diagonally | **Tokin** (T) | as a Gold ("a tokin moves the same way as a golden general") |
| **Pawn** (P) | 1 square straight forward | **Knight** (N) | shogi knight: jumps to 2-forward-±1-sideways; forward only; jumps over pieces |

Note the pairings are unique to Micro Shogi (e.g. Gold↔Rook, Silver↔Lance) but
each individual move is the orthodox shogi move for that piece.

## Promotion — by capture (the defining mechanic)

- **No promotion zone.** A piece promotes **when it captures**, and promotion is
  **mandatory** — on a capture the moving piece flips to its reverse side. — [confirmed]
- **Reversible.** "When a lance, tokin, rook, or knight makes a capture, it flips
  back to its former state. A piece can flip back and forth during the game as it
  makes captures." — [confirmed]
- **King** has no reverse; it never flips. — [confirmed]

## Drops — unrestricted (note the differences from standard shogi)

Captured pieces go to the capturer's hand and may be dropped on a vacant square as
a full move, as in standard shogi — **except all the usual restrictions are
removed:** — [confirmed]

- A player **may drop a piece with either side facing up** (promoted OR unpromoted
  face — the hand piece is face-agnostic; the face is chosen at drop time).
- **No nifu** — two unpromoted pawns on the same file IS allowed.
- **No uchifuzume** — a pawn drop MAY give immediate checkmate.
- **No last-rank restriction** — a piece may be dropped with no legal moves
  thereafter (e.g. a pawn/knight on the far rank).

> Solver implications: (1) **no drop-legality code** is needed — drops are
> unconstrained, which simplifies move-gen but **raises branching** (every hand
> piece can drop on every empty square, in either face). (2) The hand state is
> just `(type, owner)` with no face — consistent with the state-space upper-bound
> enumerator (`repro/`), which counts in-hand pieces as owner-only.

## Trapped pieces — legal

A piece can sit where it has no legal move (a Knight/Pawn/Lance stranded at a far
rank). "Any trapped piece may be captured and returned to play as part of the
opposing army." So a stalemated *single piece* is fine; this is not a loss
condition. — [confirmed]

## Winning condition — [open], default chosen

Wikipedia does **not** state the win condition. Micro Shogi is a shogi variant
with a King and orthodox piece moves, so the objective is **checkmate** (orthodox
shogi). — inferred, not sourced.

> Solver decision: use **King capture as the terminal condition** (as our Dōbutsu
> solver used Lion capture). Under optimal play this is equivalent to checkmate for
> win/loss determination, and it is simpler and unambiguous to implement. Confirm
> there is no special terminal rule (no "Try"/king-entry win as in Dōbutsu; none is
> mentioned for Micro Shogi). — see `open-questions.md`.

## Repetition (sennichite) — [open], default chosen

Wikipedia does not state a repetition rule. — not sourced.

> Solver decision (baseline): **repetition → draw**, realized exactly as in the
> Dōbutsu solver (positions unresolved after the win/loss fixpoint are draws). The
> orthodox-shogi refinement (perpetual check is a loss for the checking side) adds
> asymmetry to the fixpoint (hurdle H3 in `architecture.md`); decide whether to
> model it before the full solve. The baseline draw-rule is correct for a first
> pass and matches how Dōbutsu was solved.

## Origin / history — [open], sources conflict

- Wikipedia: English name by **Kerry Handscomb (NOST)**, who credits invention to
  **Ōyama Yasuharu** (a famous professional). A custom set existed by the mid-1980s,
  so it predates that. Exact year not stated.
- An earlier secondary source (Grokipedia) attributed it to manga artist **Fujio
  Akatsuka**, commissioned by Dai-Ichi Life. **This conflicts** with Wikipedia.
- Do not assert an inventor/date in published prose until a primary source settles
  it. — [open]

## What this resolves vs the four pre-research ambiguities

1. **Capture-flip promotion** → RESOLVED: on capture, mandatory flip; reversible.
2. **Drop restrictions** → RESOLVED: none (no nifu, no uchifuzume, no last-rank
   ban); plus drop with either face up.
3. **Repetition** → OPEN: default draw (baseline), perpetual-check refinement TBD.
4. **Last-rank / no-move legality** → RESOLVED: legal; trapped pieces allowed.

New opens surfaced: exact **win condition** statement, and the **origin** conflict.
