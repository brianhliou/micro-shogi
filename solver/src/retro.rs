//! In-RAM retrograde solver + an independent forward-minimax validator, for the
//! calibration ladder (small reduced-piece sub-games).
//!
//! The retrograde `solve` is the same Jacobi fixpoint as the Dōbutsu solver,
//! ported to the Micro Shogi engine with u128 canonical keys. `forward_check` is a
//! deliberately *independent* implementation (forward negamax with repetition =
//! draw) used to cross-validate the retrograde output — we have no external oracle
//! for Micro Shogi, so two unrelated solvers agreeing is the correctness evidence.
//! `audit` is a full-table consistency check (every value equals the minimax of its
//! children's values).

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use crate::{canonical_key, parse, unpack, Position};

/// Value convention: +d = win in d plies, -d = loss in d plies, 0 = draw/unknown.
pub struct Solved {
    pub keys: Vec<u128>,
    pub index: HashMap<u128, u32>,
    pub values: Vec<i32>,
    pub edges: u64,        // total edge-operations performed during the fixpoint
    pub rounds: u32,       // fixpoint rounds to convergence
    pub fixpoint_ns: u128, // wall-clock of the fixpoint loop (for ns/edge)
}

/// Start position for a named rung of the calibration ladder (reduced piece set,
/// real 4×5 board / engine). 180°-symmetric setups.
pub fn rung_start(name: &str) -> Option<Position> {
    let s = match name {
        "KP" => "S/k---/p---/----/---P/---K/-",
        "KPG" => "S/k-g-/p---/----/---P/-G-K/-",
        "KPGS" => "S/k-gs/p---/----/---P/SG-K/-",
        "FULL" | "KPGSB" => "S/kbgs/p---/----/---P/SGBK/-",
        _ => return None,
    };
    parse(s)
}

/// Enumerate the canonical reachable positions from `start`, assigning dense ids.
/// Terminal positions (a king capture is available) are interned but not expanded.
pub fn enumerate(start: &Position) -> (Vec<u128>, HashMap<u128, u32>) {
    let mut index: HashMap<u128, u32> = HashMap::new();
    let mut keys: Vec<u128> = Vec::new();
    let mut q: VecDeque<u128> = VecDeque::new();
    let k0 = canonical_key(start);
    index.insert(k0, 0);
    keys.push(k0);
    q.push_back(k0);
    while let Some(k) = q.pop_front() {
        let p = unpack(k);
        let ms = p.moves();
        if ms.iter().any(|m| p.is_terminal_win_move(m)) {
            continue; // winning position; no need to explore children
        }
        for m in &ms {
            let ck = canonical_key(&p.make(m));
            if !index.contains_key(&ck) {
                let id = keys.len() as u32;
                index.insert(ck, id);
                keys.push(ck);
                q.push_back(ck);
            }
        }
    }
    (keys, index)
}

/// Retrograde solve: enumerate, then a Jacobi fixpoint to distance-to-mate.
pub fn solve(start: &Position) -> Solved {
    let (keys, index) = enumerate(start);
    let n = keys.len();
    let mut values = vec![0i32; n]; // 0 = unknown (and, after convergence, draw)
    let mut unknown: Vec<u32> = Vec::new();

    for id in 0..n {
        let p = unpack(keys[id]);
        let ms = p.moves();
        if ms.iter().any(|m| p.is_terminal_win_move(m)) {
            values[id] = 1; // win in 1 (capture the king)
        } else if ms.is_empty() {
            values[id] = -2; // no legal move = loss
        } else {
            unknown.push(id as u32);
        }
    }

    let mut edges = 0u64;
    let mut rounds = 0u32;
    let t = Instant::now();
    loop {
        rounds += 1;
        let mut decisions: Vec<(u32, i32)> = Vec::new();
        let mut next: Vec<u32> = Vec::with_capacity(unknown.len() / 2);
        for &id in &unknown {
            let p = unpack(keys[id as usize]);
            let mut best_win: Option<i32> = None;
            let mut worst_loss = 0i32;
            let mut any_unknown = false;
            for m in &p.moves() {
                edges += 1;
                let v = values[*index.get(&canonical_key(&p.make(m))).unwrap() as usize];
                if v == 0 {
                    any_unknown = true;
                } else if v < 0 {
                    let d = -v;
                    best_win = Some(best_win.map_or(d, |b| b.min(d)));
                } else {
                    worst_loss = worst_loss.max(v);
                }
            }
            if let Some(d) = best_win {
                decisions.push((id, d + 1));
            } else if any_unknown {
                next.push(id);
            } else {
                decisions.push((id, -(worst_loss + 1)));
            }
        }
        let decided = decisions.len();
        for (id, v) in &decisions {
            values[*id as usize] = *v;
        }
        unknown = next;
        if decided == 0 {
            break;
        }
    }
    let fixpoint_ns = t.elapsed().as_nanos();

    Solved {
        keys,
        index,
        values,
        edges,
        rounds,
        fixpoint_ns,
    }
}

/// Full-table consistency audit: every stored value must equal the minimax of its
/// children's values (and immediate-win/no-move positions must be +1 / -2). Returns
/// the number of inconsistent positions (0 = pass).
pub fn audit(s: &Solved) -> u64 {
    let mut bad = 0u64;
    for id in 0..s.keys.len() {
        let p = unpack(s.keys[id]);
        let v = s.values[id];
        let ms = p.moves();
        let expected = if ms.iter().any(|m| p.is_terminal_win_move(m)) {
            1
        } else if ms.is_empty() {
            -2
        } else {
            let mut best_win: Option<i32> = None;
            let mut worst_loss = 0i32;
            let mut any_draw = false;
            for m in &ms {
                let cv = s.values[*s.index.get(&canonical_key(&p.make(m))).unwrap() as usize];
                if cv < 0 {
                    let d = -cv;
                    best_win = Some(best_win.map_or(d, |b| b.min(d)));
                } else if cv > 0 {
                    worst_loss = worst_loss.max(cv);
                } else {
                    any_draw = true;
                }
            }
            match best_win {
                Some(d) => d + 1,
                None if any_draw => 0,
                None => -(worst_loss + 1),
            }
        };
        if v != expected {
            bad += 1;
        }
    }
    bad
}

// ---- independent forward solver (negamax, repetition = draw) ----

#[derive(Clone, Copy, PartialEq, Eq)]
enum Wdl {
    Win,
    Loss,
    Draw,
}

fn forward(p: &Position, memo: &mut HashMap<u128, Wdl>, path: &mut HashSet<u128>) -> Wdl {
    let key = canonical_key(p);
    if let Some(&v) = memo.get(&key) {
        return v; // only decisive (Win/Loss) results are memoized — history-independent
    }
    if path.contains(&key) {
        return Wdl::Draw; // repetition on the current line
    }
    let ms = p.moves();
    if ms.iter().any(|m| p.is_terminal_win_move(m)) {
        memo.insert(key, Wdl::Win);
        return Wdl::Win;
    }
    if ms.is_empty() {
        memo.insert(key, Wdl::Loss);
        return Wdl::Loss;
    }
    path.insert(key);
    let mut any_loss = false;
    let mut all_win = true;
    for m in &ms {
        match forward(&p.make(m), memo, path) {
            Wdl::Loss => {
                any_loss = true;
                break; // a child that loses for the opponent ⇒ I win
            }
            Wdl::Win => {}
            Wdl::Draw => all_win = false,
        }
    }
    path.remove(&key);
    let res = if any_loss {
        Wdl::Win
    } else if all_win {
        Wdl::Loss
    } else {
        Wdl::Draw
    };
    if res != Wdl::Draw {
        memo.insert(key, res);
    }
    res
}

/// Cross-check every reachable position's retrograde value-sign against the
/// independent forward solver. Returns the number of mismatches (0 = validated).
pub fn forward_check(s: &Solved) -> u64 {
    let mut memo: HashMap<u128, Wdl> = HashMap::new();
    let mut mism = 0u64;
    for id in 0..s.keys.len() {
        let p = unpack(s.keys[id]);
        let mut path = HashSet::new();
        let fv = forward(&p, &mut memo, &mut path);
        let rv = s.values[id];
        let r = if rv > 0 {
            Wdl::Win
        } else if rv < 0 {
            Wdl::Loss
        } else {
            Wdl::Draw
        };
        if fv != r {
            mism += 1;
        }
    }
    mism
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kp_solves_and_audits() {
        let start = rung_start("KP").unwrap();
        let s = solve(&start);
        assert!(s.keys.len() > 1);
        assert_eq!(audit(&s), 0, "consistency audit must pass");
    }

    #[test]
    fn mirror_start_same_value() {
        // solving from the start and from its canonical form give the same root value
        let start = rung_start("KP").unwrap();
        let s = solve(&start);
        assert_eq!(s.values[0], solve(&unpack(canonical_key(&start))).values[0]);
    }
}
