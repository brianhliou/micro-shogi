#!/usr/bin/env python3
"""Exact 'all-arrangements' upper bound on positions for drop-shogi variants,
computed Tanaka-2009 Table-1 style.

We FIRST reproduce dobutsu's published upper bound 1,567,925,964 (and its
by-pieces-in-hand breakdown) to validate the combinatorial model, THEN apply the
identical model to micro shogi (4x5) and minishogi (5x5).

Model (verified to reproduce Tanaka's dobutsu sub-counts exactly):
- Kings: exactly one per player, always on the board (capturing a king ends the
  game), so they are never in hand.
- Every other ("capturable") piece type has 2 copies total, freely distributable
  by owner. Each copy is either on the board or in a player's hand.
- On-board state of an occupied square = owner (2) x face (2 if the type can
  promote/flip, else 1).
- In-hand pieces revert to base (no face) and identical in-hand pieces of one
  type collapse to counts, giving (h+1) owner-splits for h copies in hand.
- Sum over every split of each type's 2 copies into (on_board, in_hand).

This counts arrangements ignoring legality and reachability -> a strict upper
bound. Reachable/legal counts are bracketed from this via calibrated ratios in
the printed summary.
"""
from math import comb
from itertools import product


def upper_bound(squares, capturable):
    """capturable: list of (name, promotes: bool). Returns (total, by_in_hand)."""
    by_hand = {}
    total = 0
    for js in product(range(3), repeat=len(capturable)):  # on-board count per type
        on_board = 2 + sum(js)  # +2 kings
        if on_board > squares:
            continue
        ways = 1
        rem = squares
        # kings: choose 2 squares, assign P1/P2 (exactly one each) -> x2
        ways *= comb(rem, 2) * 2
        rem -= 2
        for (_, promotes), j in zip(capturable, js):
            ways *= comb(rem, j) * (2 * (2 if promotes else 1)) ** j
            rem -= j
        hand = 1
        for j in js:
            hand *= (2 - j) + 1  # owner-splits for the (2-j) copies in hand
        cnt = ways * hand
        tih = sum(2 - j for j in js)
        by_hand[tih] = by_hand.get(tih, 0) + cnt
        total += cnt
    return total, by_hand


GAMES = {
    "dobutsu (3x4)": (
        12,
        [("giraffe", False), ("elephant", False), ("chick", True)],
    ),
    "micro shogi (4x5)": (
        20,
        # gold<->rook, silver<->lance, bishop<->tokin, pawn<->knight: all flip
        [("gold", True), ("silver", True), ("bishop", True), ("pawn", True)],
    ),
    "minishogi (5x5)": (
        25,
        # gold does not promote in minishogi; the rest do
        [("rook", True), ("bishop", True), ("gold", False),
         ("silver", True), ("pawn", True)],
    ),
}

DOBUTSU_PUBLISHED = 1_567_925_964
DOBUTSU_BREAKDOWN = {0: 638_668_800, 1: 638_668_800, 2: 242_161_920,
                     3: 44_098_560, 4: 4_134_240, 5: 190_080, 6: 3_564}
DOBUTSU_REACHABLE = 246_803_167          # Tanaka, reachable from start
DOBUTSU_CANONICAL = 213_993_386          # our solver, turn+LR folded
MINISHOGI_REACHABLE = 2.38e18            # arXiv 2409.00129 estimate


def main():
    results = {}
    for name, (sq, cap) in GAMES.items():
        total, bh = upper_bound(sq, cap)
        results[name] = total
        print(f"\n=== {name} : {sq} squares ===")
        print(f"all-arrangements upper bound = {total:,}")
        for k in sorted(bh):
            print(f"  {k} in hand: {bh[k]:,}")

    # ---- validation ----
    d = results["dobutsu (3x4)"]
    print("\n=== validation vs Tanaka 2009 ===")
    print(f"dobutsu computed   {d:,}")
    print(f"dobutsu published  {DOBUTSU_PUBLISHED:,}")
    print("MATCH" if d == DOBUTSU_PUBLISHED else "*** MISMATCH ***")
    _, dbh = upper_bound(12, GAMES["dobutsu (3x4)"][1])
    print("breakdown match:",
          all(dbh.get(k) == v for k, v in DOBUTSU_BREAKDOWN.items()))

    # ratios calibrated on dobutsu (Tanaka-internal denominators)
    r_reach = DOBUTSU_REACHABLE / DOBUTSU_PUBLISHED          # reachable / upper
    r_canon = DOBUTSU_CANONICAL / DOBUTSU_REACHABLE          # our-fold / reachable
    print(f"\ndobutsu reachable/upper = {r_reach:.4f}")
    print(f"dobutsu canonical/reachable = {r_canon:.4f}")

    # cross-check the reachable/upper ratio on minishogi
    mini = results["minishogi (5x5)"]
    print(f"\nminishogi upper = {mini:.3e}")
    print(f"minishogi reachable (arXiv est) = {MINISHOGI_REACHABLE:.3e}")
    print(f"minishogi reachable/upper = {MINISHOGI_REACHABLE / mini:.4f}")

    # ---- micro shogi projection ----
    micro = results["micro shogi (4x5)"]
    print("\n=== micro shogi (4x5) projection ===")
    print(f"upper bound (exact)         {micro:,}")
    lo = MINISHOGI_REACHABLE / mini
    hi = r_reach
    print(f"reachable bracket  ~{micro*min(lo,hi):.3e} .. {micro*max(lo,hi):.3e}")
    reach_mid = micro * r_reach
    canon_mid = reach_mid * r_canon
    print(f"reachable (dobutsu ratio)   {reach_mid:.3e}")
    print(f"canonical (our-fold)        {canon_mid:.3e}")

    # storage at a few packings (canonical, complete tablebase)
    print("\nstorage for complete canonical tablebase:")
    for bits, label in [(2, "W/L/D only"), (10, "DTM ~10 bit"),
                        (16, "DTM+slack 16 bit")]:
        tb = canon_mid * bits / 8 / 1e12
        print(f"  {label:18s} {tb:8.1f} TB")


if __name__ == "__main__":
    main()
