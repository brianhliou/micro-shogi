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

/// Push-based retrograde: BFS outward from resolved positions, each edge touched
/// once. A position wins as soon as one child is a loss; loses when its last child
/// resolves to a win (counter hits 0); a position with a drawing child never has
/// its counter reach 0 and is left a draw. Unlike the Jacobi `solve`, this never
/// re-scans a draw — essential for drop-shogi, which is ~70% draws.
///
/// Reverse adjacency (predecessors) is built by transposing the forward edges we
/// can already generate — no unmove-generator needed for the in-RAM calibration.
/// Produces values identical to `solve` (cross-checked); use this one at scale.
pub fn solve_push(start: &Position) -> Solved {
    let (keys, index) = enumerate(start);
    let n = keys.len();
    let mut value = vec![0i32; n]; // 0 = unknown/draw
    let mut nchild = vec![0u32; n];
    let mut indeg = vec![0u32; n];
    let mut resolved_no_children = vec![false; n]; // terminal-win or no-move
    let mut queue: VecDeque<u32> = VecDeque::new();

    // pass 1: classify, count children, accumulate in-degrees
    for id in 0..n {
        let p = unpack(keys[id]);
        let ms = p.moves();
        if ms.iter().any(|m| p.is_terminal_win_move(m)) {
            value[id] = 1; // win in 1 (capture the king)
            resolved_no_children[id] = true;
            queue.push_back(id as u32);
            continue;
        }
        if ms.is_empty() {
            value[id] = -2; // no legal move = loss
            resolved_no_children[id] = true;
            queue.push_back(id as u32);
            continue;
        }
        for m in &ms {
            let cid = *index.get(&canonical_key(&p.make(m))).unwrap();
            indeg[cid as usize] += 1;
        }
        nchild[id] = ms.len() as u32;
    }

    // CSR for predecessors (transpose of forward edges)
    let mut par_off = vec![0u32; n + 1];
    for i in 0..n {
        par_off[i + 1] = par_off[i] + indeg[i];
    }
    let total_edges = par_off[n] as usize;
    let mut par_idx = vec![0u32; total_edges];
    let mut fill: Vec<u32> = par_off[..n].to_vec();
    // pass 2: fill predecessor lists
    for id in 0..n {
        if resolved_no_children[id] {
            continue;
        }
        let p = unpack(keys[id]);
        for m in &p.moves() {
            let cid = *index.get(&canonical_key(&p.make(m))).unwrap() as usize;
            par_idx[fill[cid] as usize] = id as u32;
            fill[cid] += 1;
        }
    }
    drop(fill);
    drop(indeg);

    // BFS propagation: each edge processed once
    let mut cnt = nchild;
    let mut edges = 0u64;
    let t = Instant::now();
    while let Some(cid) = queue.pop_front() {
        let cv = value[cid as usize];
        let lo = par_off[cid as usize] as usize;
        let hi = par_off[cid as usize + 1] as usize;
        for &pid in &par_idx[lo..hi] {
            edges += 1;
            let pid = pid as usize;
            if value[pid] != 0 {
                continue; // already resolved
            }
            if cv < 0 {
                // child is a loss for its mover (parent's opponent) ⇒ parent wins
                value[pid] = -cv + 1;
                queue.push_back(pid as u32);
            } else {
                // child is a win for the opponent ⇒ doesn't help the parent
                cnt[pid] -= 1;
                if cnt[pid] == 0 {
                    // all children win for the opponent ⇒ parent loses; this last
                    // (max-distance) child sets the loss distance
                    value[pid] = -(cv + 1);
                    queue.push_back(pid as u32);
                }
            }
        }
    }
    let fixpoint_ns = t.elapsed().as_nanos();

    Solved {
        keys,
        index,
        values: value,
        edges,
        rounds: 0, // not applicable to the push method
        fixpoint_ns,
    }
}

/// Compare two solves position-by-position (same start ⇒ same deterministic ids).
/// Returns the number of value mismatches (0 = identical).
pub fn cross_check(a: &Solved, b: &Solved) -> u64 {
    assert_eq!(a.keys.len(), b.keys.len(), "different position counts");
    let mut mism = 0u64;
    for i in 0..a.keys.len() {
        debug_assert_eq!(a.keys[i], b.keys[i], "enumeration order diverged");
        if a.values[i] != b.values[i] {
            mism += 1;
        }
    }
    mism
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
