# Micro Shogi — ruleset (the solver's spec)

> Milestone #1, the gate. The solver is built against THIS document. Tags:
> **[confirmed]** (stated by the best available source, Wikipedia *Micro shogi*),
> **[standard]** (Wikipedia says pieces "move like their shogi equivalents"; the
> movement is the standard shogi definition for that piece name), **[open]**
> (under-documented; a default is chosen and flagged for verification).

**Sources & verification.** Corroborated by **two independent encyclopedic sources**
that agree on every substantive rule:
- English Wikipedia, *Micro shogi* — <https://en.wikipedia.org/wiki/Micro_shogi>
- Japanese Wikipedia, *五分摩訶将棋* (gofun maka shōgi) —
  <https://ja.wikipedia.org/wiki/五分摩訶将棋>

Residual gaps (neither source closes them): **per-piece movement** is given only as
"pieces move like their shogi equivalents" (no explicit per-piece text anywhere
found — we use the standard shogi move for each named piece), the **king-flip-on-
capture** edge case (inferred from "king does not promote"), and **repetition**
(undocumented in both). The 100% check on moves would be a code implementation
(Fairy-Stockfish / hachu / Jocly variant config) or a primary source (Ochi
Nobuyoshi's book, which introduced the game). See `open-questions.md`.

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
removed:** — [confirmed, 2 sources; JA states 二歩・打ち歩詰めは禁止されていない verbatim]

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

## Winning condition — [confirmed: checkmate]

The objective is **checkmate**, like standard shogi — JA Wikipedia: "相手の玉将を
詰めたほうが勝ちである" (the side that checkmates the opponent's king wins). No special
terminal rule (no Dōbutsu-style "Try"/king-entry win) is mentioned by either source.

> Solver implementation: use **King capture as the terminal condition** (as our
> Dōbutsu solver used Lion capture). Under optimal play this is equivalent to
> checkmate for win/loss determination, and is simpler and unambiguous.

## Repetition (sennichite) — [open], default chosen

Wikipedia does not state a repetition rule. — not sourced.

> Solver decision (baseline): **repetition → draw**, realized exactly as in the
> Dōbutsu solver (positions unresolved after the win/loss fixpoint are draws). The
> orthodox-shogi refinement (perpetual check is a loss for the checking side) adds
> asymmetry to the fixpoint (hurdle H3 in `architecture.md`); decide whether to
> model it before the full solve. The baseline draw-rule is correct for a first
> pass and matches how Dōbutsu was solved.

## Origin / history — [resolved: Ōyama]

- **Both** EN and JA Wikipedia credit invention to **Ōyama Yasuharu** (a famous
  professional); introduced in Ochi Nobuyoshi's book; **existed by 1982**; English
  name coined by **Kerry Handscomb (NOST)**.
- The earlier "Fujio Akatsuka" attribution (Grokipedia, a single weak source) is
  contradicted by both encyclopedias and is treated as **erroneous**.
- Still cite a primary source before publishing the exact year. — [minor]

## Status vs the pre-research ambiguities (after 2-source verification)

1. **Capture-flip promotion** → RESOLVED (2 sources): on capture, mandatory flip; reversible.
2. **Drop restrictions** → RESOLVED (2 sources): none (no nifu/uchifuzume/last-rank ban);
   droppable on either face.
3. **Last-rank / no-move legality** → RESOLVED: legal; trapped pieces allowed.
4. **Winning condition** → RESOLVED (JA Wikipedia): checkmate (King-capture terminal).
5. **Origin** → RESOLVED: Ōyama (both sources); Akatsuka claim erroneous.

**Still open** (do not block the build; engine uses defaults):
- **Repetition** — undocumented in both sources; baseline = draw.
- **Per-piece moves** — sourced only as "standard shogi equivalents"; verify against a code
  implementation (Fairy-Stockfish / hachu / Jocly) or a primary source for 100% certainty.
- **King flip on capture** — inferred (king never promotes ⇒ never flips); very low risk.
