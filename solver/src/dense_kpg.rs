//! KPG-specific dense-rank solver.
//!
//! This is deliberately narrower than the general Micro Shogi engine: it ranks
//! exactly the K+P+G material slice. It keeps the rank domain as raw Sente-to-move
//! arrangements and folds left-right mirror by always using the lower raw rank.
//! That avoids a reachable-key `HashMap` and avoids storing the predecessor CSR.

#[cfg(test)]
use std::collections::HashSet;
use std::time::Instant;

use crate::{canonical_turn, parse, Kind, Owner, Position, NFILE, NRANK, NSQ};

const TOKENS_PER_KIND: u8 = 2;
const GOLD_HAND: usize = 0;
const PAWN_HAND: usize = 3;
const REACHABLE_TARGET: usize = 869_287_068;

#[derive(Clone, Debug)]
pub struct DenseKpgSolved {
    pub start_id: u32,
    pub domain_size: u32,
    pub positions: usize,
    pub start_value: i16,
    pub wins: u64,
    pub losses: u64,
    pub draws: u64,
    pub max_dtm: i16,
    pub total_moves: u64,
    pub propagation_edges: u64,
    pub predecessor_candidates: u64,
    pub predecessor_ids: u64,
    pub duplicate_predecessor_ids: u64,
    pub enumeration_secs: f64,
    pub classification_secs: f64,
    pub propagation_secs: f64,
    pub stats_secs: f64,
    pub audit_bad_positions: Option<u64>,
    pub audit_secs: Option<f64>,
}

pub fn kpg_start() -> Position {
    parse("S/k-g-/p---/----/---P/-G-K/-").expect("valid KPG start")
}

pub const fn domain_size() -> u32 {
    (NSQ as u32) * ((NSQ as u32) - 1) * per_king_size()
}

const fn per_king_size() -> u32 {
    let e = (NSQ as u32) - 2;
    3 * kind_domain_size(e)
        + 8 * e * kind_domain_size(e - 1)
        + 16 * choose2(e) * kind_domain_size(e - 2)
}

const fn kind_domain_size(empty: u32) -> u32 {
    3 + 8 * empty + 16 * choose2(empty)
}

const fn choose2(n: u32) -> u32 {
    n * (n - 1) / 2
}

pub fn rank_canonical(p: &Position) -> Option<u32> {
    let q = canonical_turn(p);
    let r1 = rank_raw_sente(&q)?;
    let mirror = mirror_sente(&q);
    let r2 = rank_raw_sente(&mirror)?;
    Some(r1.min(r2))
}

pub fn rank_raw_sente(p: &Position) -> Option<u32> {
    if p.turn != Owner::Sente {
        return None;
    }
    if p.hand_sente[1] != 0 || p.hand_sente[2] != 0 {
        return None;
    }
    if p.hand_gote[1] != 0 || p.hand_gote[2] != 0 {
        return None;
    }

    let mut sk = None;
    let mut gk = None;
    let mut occupied = 0u32;
    for sq in 0..NSQ {
        let Some((kind, _, owner)) = p.board[sq] else {
            continue;
        };
        occupied |= 1u32 << sq;
        match (kind, owner) {
            (Kind::King, Owner::Sente) if sk.is_none() => sk = Some(sq as u8),
            (Kind::King, Owner::Gote) if gk.is_none() => gk = Some(sq as u8),
            (Kind::King, _) => return None,
            (Kind::Gold | Kind::Pawn, _) => {}
            _ => return None,
        }
    }
    let sk = sk?;
    let gk = gk?;
    if sk == gk {
        return None;
    }

    let king_rank =
        (sk as u32) * ((NSQ as u32) - 1) + if gk < sk { gk as u32 } else { gk as u32 - 1 };
    let king_mask = (1u32 << sk) | (1u32 << gk);
    let empty_after_kings = empty_squares(king_mask);

    let gold = rank_kind_placement(p, Kind::Gold, GOLD_HAND, &empty_after_kings)?;
    let gold_weighted = weighted_kind_rank(empty_after_kings.len() as u32, &gold)?;
    let gold_mask = placement_mask(&empty_after_kings, &gold);
    let empty_after_gold = empty_squares(king_mask | gold_mask);

    let pawn = rank_kind_placement(p, Kind::Pawn, PAWN_HAND, &empty_after_gold)?;
    let pawn_rank = pawn.rank;

    if occupied != king_mask | gold_mask | placement_mask(&empty_after_gold, &pawn) {
        return None;
    }

    Some(king_rank * per_king_size() + gold_weighted + pawn_rank)
}

pub fn unrank_raw_sente(id: u32) -> Position {
    assert!(id < domain_size(), "KPG rank out of range: {id}");
    let per_king = per_king_size();
    let king_rank = id / per_king;
    let mut rem = id % per_king;

    let sk = king_rank / ((NSQ as u32) - 1);
    let g_idx = king_rank % ((NSQ as u32) - 1);
    let gk = if g_idx >= sk { g_idx + 1 } else { g_idx };

    let mut p = empty_position();
    p.board[sk as usize] = Some((Kind::King, false, Owner::Sente));
    p.board[gk as usize] = Some((Kind::King, false, Owner::Gote));

    let king_mask = (1u32 << sk) | (1u32 << gk);
    let empty_after_kings = empty_squares(king_mask);
    let (gold, pawn_remainder) = unrank_weighted_kind(empty_after_kings.len() as u32, rem);
    rem = pawn_remainder;
    apply_placement(&mut p, Kind::Gold, GOLD_HAND, &empty_after_kings, &gold);

    let gold_mask = placement_mask(&empty_after_kings, &gold);
    let empty_after_gold = empty_squares(king_mask | gold_mask);
    let pawn = unrank_kind(empty_after_gold.len() as u32, rem);
    apply_placement(&mut p, Kind::Pawn, PAWN_HAND, &empty_after_gold, &pawn);
    p
}

pub fn predecessor_ids(child_id: u32) -> PredecessorIds {
    let child = unrank_raw_sente(child_id);
    let mut candidates = 0u64;
    let mut ids = Vec::with_capacity(64);
    let raw_child = rotate_swap_to_gote(&child);
    predecessor_candidates_raw(&raw_child, &mut candidates, &mut ids);
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    let after = ids.len();
    PredecessorIds {
        ids,
        candidates,
        duplicates: (before - after) as u64,
    }
}

pub fn predecessor_ids_verified(child_id: u32) -> PredecessorIds {
    let mut preds = predecessor_ids(child_id);
    preds.ids.retain(|&pid| {
        let parent = unrank_raw_sente(pid);
        parent.moves().into_iter().any(|m| {
            !parent.is_terminal_win_move(&m) && rank_canonical(&parent.make(&m)) == Some(child_id)
        })
    });
    preds
}

#[cfg(test)]
fn predecessor_ids_both_mirrors(child_id: u32) -> PredecessorIds {
    let child = unrank_raw_sente(child_id);
    let mut candidates = 0u64;
    let mut ids = Vec::with_capacity(64);
    let mut seen_raw_children = HashSet::with_capacity(2);
    for q in [child, mirror_sente(&child)] {
        let raw_child = rotate_swap_to_gote(&q);
        if !seen_raw_children.insert(crate::pack(&raw_child)) {
            continue;
        }
        predecessor_candidates_raw(&raw_child, &mut candidates, &mut ids);
    }
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    let after = ids.len();
    PredecessorIds {
        ids,
        candidates,
        duplicates: (before - after) as u64,
    }
}

#[derive(Debug)]
pub struct PredecessorIds {
    pub ids: Vec<u32>,
    pub candidates: u64,
    pub duplicates: u64,
}

pub fn solve_dense_kpg(audit_large: bool) -> DenseKpgSolved {
    let start = kpg_start();
    let start_id = rank_canonical(&start).expect("KPG start is rankable");
    let domain = domain_size();
    let domain_usize = domain as usize;
    let total_start = Instant::now();

    let enum_start = Instant::now();
    let mut reachable = BitSet::new(domain_usize);
    let mut ids = Vec::<u32>::with_capacity(REACHABLE_TARGET);
    reachable.set(start_id);
    ids.push(start_id);
    let mut head = 0usize;
    let mut last_progress = Instant::now();
    while head < ids.len() {
        let id = ids[head];
        head += 1;
        if last_progress.elapsed().as_secs() >= 30 {
            eprintln!(
                "dense-kpg enumerate: scanned={} queued={} domain={}",
                head,
                ids.len(),
                domain
            );
            last_progress = Instant::now();
        }
        let p = unrank_raw_sente(id);
        let moves = p.moves();
        if moves.iter().any(|m| p.is_terminal_win_move(m)) {
            continue;
        }
        for m in &moves {
            let cid = rank_canonical(&p.make(m)).expect("KPG child is rankable");
            if !reachable.get(cid) {
                reachable.set(cid);
                ids.push(cid);
            }
        }
    }
    let enumeration_secs = enum_start.elapsed().as_secs_f64();

    let classify_start = Instant::now();
    let mut values = vec![0i16; domain_usize];
    let mut remaining_children = vec![0u8; domain_usize];
    let mut queue = Vec::<u32>::new();
    let mut total_moves = 0u64;
    last_progress = Instant::now();
    for (i, &id) in ids.iter().enumerate() {
        if last_progress.elapsed().as_secs() >= 30 {
            eprintln!(
                "dense-kpg classify: scanned={}/{} resolved_queue={}",
                i,
                ids.len(),
                queue.len()
            );
            last_progress = Instant::now();
        }
        let p = unrank_raw_sente(id);
        let moves = p.moves();
        total_moves += moves.len() as u64;
        if moves.iter().any(|m| p.is_terminal_win_move(m)) {
            values[id as usize] = 1;
            queue.push(id);
        } else if moves.is_empty() {
            values[id as usize] = -2;
            queue.push(id);
        } else {
            remaining_children[id as usize] =
                u8::try_from(moves.len()).expect("KPG position has more than 255 legal moves");
        }
    }
    drop(ids);
    let classification_secs = classify_start.elapsed().as_secs_f64();

    let propagation_start = Instant::now();
    let mut propagation_edges = 0u64;
    let mut predecessor_candidates = 0u64;
    let mut predecessor_id_count = 0u64;
    let mut duplicate_predecessor_ids = 0u64;
    let mut qhead = 0usize;
    last_progress = Instant::now();
    while qhead < queue.len() {
        let cid = queue[qhead];
        qhead += 1;
        if last_progress.elapsed().as_secs() >= 30 {
            eprintln!(
                "dense-kpg propagate: popped={} queue={} updates={} pred_ids={}",
                qhead,
                queue.len(),
                propagation_edges,
                predecessor_id_count
            );
            last_progress = Instant::now();
        }
        let cv = values[cid as usize];
        let preds = predecessor_ids(cid);
        predecessor_candidates += preds.candidates;
        predecessor_id_count += preds.ids.len() as u64;
        duplicate_predecessor_ids += preds.duplicates;
        for pid in preds.ids {
            if !reachable.get(pid) {
                continue;
            }
            if values[pid as usize] != 0 {
                continue;
            }
            propagation_edges += 1;
            if cv < 0 {
                values[pid as usize] = -cv + 1;
                queue.push(pid);
            } else {
                let cnt = &mut remaining_children[pid as usize];
                debug_assert!(*cnt > 0, "unresolved reachable parent had no child count");
                *cnt -= 1;
                if *cnt == 0 {
                    values[pid as usize] = -(cv + 1);
                    queue.push(pid);
                }
            }
        }
    }
    let propagation_secs = propagation_start.elapsed().as_secs_f64();

    let stats_start = Instant::now();
    let mut positions = 0usize;
    let mut wins = 0u64;
    let mut losses = 0u64;
    let mut draws = 0u64;
    let mut max_dtm = 0i16;
    last_progress = Instant::now();
    for id in 0..domain {
        if last_progress.elapsed().as_secs() >= 30 {
            eprintln!("dense-kpg stats: scanned={id}/{domain} reachable={positions}");
            last_progress = Instant::now();
        }
        if !reachable.get(id) {
            continue;
        }
        positions += 1;
        let value = values[id as usize];
        if value > 0 {
            wins += 1;
            max_dtm = max_dtm.max(value);
        } else if value < 0 {
            losses += 1;
            max_dtm = max_dtm.max(-value);
        } else {
            draws += 1;
        }
    }
    let stats_secs = stats_start.elapsed().as_secs_f64();

    let (audit_bad_positions, audit_secs) = if audit_large {
        let audit_start = Instant::now();
        let bad = audit_dense(&reachable, &values);
        (Some(bad), Some(audit_start.elapsed().as_secs_f64()))
    } else {
        (None, None)
    };

    let _total_secs = total_start.elapsed().as_secs_f64();
    DenseKpgSolved {
        start_id,
        domain_size: domain,
        positions,
        start_value: values[start_id as usize],
        wins,
        losses,
        draws,
        max_dtm,
        total_moves,
        propagation_edges,
        predecessor_candidates,
        predecessor_ids: predecessor_id_count,
        duplicate_predecessor_ids,
        enumeration_secs,
        classification_secs,
        propagation_secs,
        stats_secs,
        audit_bad_positions,
        audit_secs,
    }
}

pub fn audit_dense(reachable: &BitSet, values: &[i16]) -> u64 {
    let mut bad = 0u64;
    for id in 0..domain_size() {
        if !reachable.get(id) {
            continue;
        }
        let p = unrank_raw_sente(id);
        let v = values[id as usize];
        let moves = p.moves();
        let expected = if moves.iter().any(|m| p.is_terminal_win_move(m)) {
            1
        } else if moves.is_empty() {
            -2
        } else {
            let mut best_win: Option<i16> = None;
            let mut worst_loss = 0i16;
            let mut any_draw = false;
            for m in &moves {
                let cid = rank_canonical(&p.make(m)).expect("KPG child is rankable");
                let cv = values[cid as usize];
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

#[derive(Clone, Debug)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    pub fn new(bits: usize) -> Self {
        Self {
            words: vec![0; bits.div_ceil(64)],
        }
    }

    pub fn get(&self, bit: u32) -> bool {
        let bit = bit as usize;
        (self.words[bit / 64] >> (bit % 64)) & 1 == 1
    }

    pub fn set(&mut self, bit: u32) {
        let bit = bit as usize;
        self.words[bit / 64] |= 1u64 << (bit % 64);
    }
}

#[derive(Clone, Debug)]
struct KindPlacement {
    rank: u32,
    board_count: u8,
    board: [(u8, u8); 2], // (index into current empty-squares list, variant)
    hand_sente: u8,
    hand_gote: u8,
}

fn rank_kind_placement(
    p: &Position,
    kind: Kind,
    hand_index: usize,
    empty: &[u8],
) -> Option<KindPlacement> {
    let mut index_of_square = [u8::MAX; NSQ];
    for (i, &sq) in empty.iter().enumerate() {
        index_of_square[sq as usize] = i as u8;
    }

    let mut board = [(0u8, 0u8); 2];
    let mut board_count = 0usize;
    for sq in 0..NSQ {
        let Some((cell_kind, promo, owner)) = p.board[sq] else {
            continue;
        };
        if cell_kind != kind {
            continue;
        }
        let idx = index_of_square[sq];
        if idx == u8::MAX || board_count == 2 {
            return None;
        }
        board[board_count] = (idx, variant(owner, promo));
        board_count += 1;
    }
    board[..board_count].sort_unstable_by_key(|&(idx, _)| idx);

    let hand_sente = p.hand_sente[hand_index];
    let hand_gote = p.hand_gote[hand_index];
    if board_count as u8 + hand_sente + hand_gote != TOKENS_PER_KIND {
        return None;
    }

    let e = empty.len() as u32;
    let rank = match board_count {
        0 => hand_sente as u32,
        1 => {
            let hand_owner = match (hand_sente, hand_gote) {
                (1, 0) => 0,
                (0, 1) => 1,
                _ => return None,
            };
            3 + ((board[0].0 as u32 * 4 + board[0].1 as u32) * 2 + hand_owner)
        }
        2 => {
            if hand_sente != 0 || hand_gote != 0 {
                return None;
            }
            let i = board[0].0 as u32;
            let j = board[1].0 as u32;
            if i >= j || j >= e {
                return None;
            }
            3 + 8 * e + pair_rank(e, i, j) * 16 + board[0].1 as u32 * 4 + board[1].1 as u32
        }
        _ => return None,
    };

    Some(KindPlacement {
        rank,
        board_count: board_count as u8,
        board,
        hand_sente,
        hand_gote,
    })
}

fn weighted_kind_rank(empty: u32, placement: &KindPlacement) -> Option<u32> {
    let b0 = 3 * kind_domain_size(empty);
    let b1 = 8 * empty * kind_domain_size(empty - 1);
    match placement.board_count {
        0 => Some(placement.rank * kind_domain_size(empty)),
        1 => Some(b0 + (placement.rank - 3) * kind_domain_size(empty - 1)),
        2 => Some(b0 + b1 + (placement.rank - 3 - 8 * empty) * kind_domain_size(empty - 2)),
        _ => None,
    }
}

fn unrank_weighted_kind(empty: u32, rank: u32) -> (KindPlacement, u32) {
    let b0_unit = kind_domain_size(empty);
    let b0 = 3 * b0_unit;
    if rank < b0 {
        return (unrank_kind(empty, rank / b0_unit), rank % b0_unit);
    }
    let rank = rank - b0;
    let b1_unit = kind_domain_size(empty - 1);
    let b1 = 8 * empty * b1_unit;
    if rank < b1 {
        return (unrank_kind(empty, 3 + rank / b1_unit), rank % b1_unit);
    }
    let rank = rank - b1;
    let b2_unit = kind_domain_size(empty - 2);
    (
        unrank_kind(empty, 3 + 8 * empty + rank / b2_unit),
        rank % b2_unit,
    )
}

fn unrank_kind(empty: u32, rank: u32) -> KindPlacement {
    assert!(rank < kind_domain_size(empty));
    if rank < 3 {
        let hand_sente = rank as u8;
        return KindPlacement {
            rank,
            board_count: 0,
            board: [(0, 0); 2],
            hand_sente,
            hand_gote: TOKENS_PER_KIND - hand_sente,
        };
    }
    let r = rank - 3;
    let b1 = 8 * empty;
    if r < b1 {
        let square_variant_hand = r;
        let hand_owner = (square_variant_hand % 2) as u8;
        let square_variant = square_variant_hand / 2;
        return KindPlacement {
            rank,
            board_count: 1,
            board: [
                ((square_variant / 4) as u8, (square_variant % 4) as u8),
                (0, 0),
            ],
            hand_sente: if hand_owner == 0 { 1 } else { 0 },
            hand_gote: if hand_owner == 1 { 1 } else { 0 },
        };
    }
    let r = r - b1;
    let pair = r / 16;
    let variants = r % 16;
    let (i, j) = unrank_pair(empty, pair);
    KindPlacement {
        rank,
        board_count: 2,
        board: [
            (i as u8, (variants / 4) as u8),
            (j as u8, (variants % 4) as u8),
        ],
        hand_sente: 0,
        hand_gote: 0,
    }
}

fn pair_rank(empty: u32, i: u32, j: u32) -> u32 {
    i * (2 * empty - i - 1) / 2 + (j - i - 1)
}

fn unrank_pair(empty: u32, mut rank: u32) -> (u32, u32) {
    for i in 0..empty {
        let row = empty - i - 1;
        if rank < row {
            return (i, i + 1 + rank);
        }
        rank -= row;
    }
    unreachable!("pair rank out of range")
}

fn placement_mask(empty: &[u8], placement: &KindPlacement) -> u32 {
    let mut mask = 0u32;
    for i in 0..placement.board_count as usize {
        mask |= 1u32 << empty[placement.board[i].0 as usize];
    }
    mask
}

fn apply_placement(
    p: &mut Position,
    kind: Kind,
    hand_index: usize,
    empty: &[u8],
    placement: &KindPlacement,
) {
    p.hand_sente[hand_index] = placement.hand_sente;
    p.hand_gote[hand_index] = placement.hand_gote;
    for i in 0..placement.board_count as usize {
        let (owner, promo) = unvariant(placement.board[i].1);
        p.board[empty[placement.board[i].0 as usize] as usize] = Some((kind, promo, owner));
    }
}

fn empty_squares(occupied: u32) -> Vec<u8> {
    (0..NSQ as u8)
        .filter(|&sq| (occupied >> sq) & 1 == 0)
        .collect()
}

fn empty_position() -> Position {
    Position {
        board: [None; NSQ],
        hand_sente: [0; 4],
        hand_gote: [0; 4],
        turn: Owner::Sente,
    }
}

fn variant(owner: Owner, promo: bool) -> u8 {
    match (owner, promo) {
        (Owner::Sente, false) => 0,
        (Owner::Sente, true) => 1,
        (Owner::Gote, false) => 2,
        (Owner::Gote, true) => 3,
    }
}

fn unvariant(v: u8) -> (Owner, bool) {
    match v {
        0 => (Owner::Sente, false),
        1 => (Owner::Sente, true),
        2 => (Owner::Gote, false),
        3 => (Owner::Gote, true),
        _ => unreachable!("bad variant"),
    }
}

fn mirror_sente(p: &Position) -> Position {
    let mut board = [None; NSQ];
    for s in 0..NSQ {
        let r = s as i8 / NFILE;
        let f = s as i8 % NFILE;
        let s2 = (r * NFILE + (NFILE - 1 - f)) as usize;
        board[s2] = p.board[s];
    }
    Position {
        board,
        hand_sente: p.hand_sente,
        hand_gote: p.hand_gote,
        turn: p.turn,
    }
}

fn rotate_swap_to_gote(p: &Position) -> Position {
    debug_assert_eq!(p.turn, Owner::Sente);
    let mut board = [None; NSQ];
    for s in 0..NSQ {
        let r = s as i8 / NFILE;
        let f = s as i8 % NFILE;
        let s2 = (((NRANK - 1 - r) * NFILE) + (NFILE - 1 - f)) as usize;
        board[s] = p.board[s2].map(|(kind, promo, owner)| (kind, promo, owner.flip()));
    }
    Position {
        board,
        hand_sente: p.hand_gote,
        hand_gote: p.hand_sente,
        turn: Owner::Gote,
    }
}

fn predecessor_candidates_raw(raw_child: &Position, candidates: &mut u64, ids: &mut Vec<u32>) {
    debug_assert_eq!(raw_child.turn, Owner::Gote);
    for to in 0..NSQ as u8 {
        let Some((kind, promo, owner)) = raw_child.board[to as usize] else {
            continue;
        };
        if owner != Owner::Sente {
            continue;
        }

        if kind != Kind::King {
            *candidates += 1;
            let mut parent = *raw_child;
            parent.turn = Owner::Sente;
            parent.board[to as usize] = None;
            parent.hand_sente[hand_index(kind)] += 1;
            push_ranked_parent(parent, ids);
        }

        for from in source_squares(raw_child, to, kind, promo, Owner::Sente) {
            *candidates += 1;
            let mut parent = *raw_child;
            parent.turn = Owner::Sente;
            parent.board[to as usize] = None;
            parent.board[from as usize] = Some((kind, promo, Owner::Sente));
            push_ranked_parent(parent, ids);
        }

        let old_promo = if kind == Kind::King { false } else { !promo };
        for from in source_squares(raw_child, to, kind, old_promo, Owner::Sente) {
            for captured_kind in [Kind::Gold, Kind::Pawn] {
                let hi = hand_index(captured_kind);
                if raw_child.hand_sente[hi] == 0 {
                    continue;
                }
                for captured_promo in [false, true] {
                    *candidates += 1;
                    let mut parent = *raw_child;
                    parent.turn = Owner::Sente;
                    parent.hand_sente[hi] -= 1;
                    parent.board[to as usize] = Some((captured_kind, captured_promo, Owner::Gote));
                    parent.board[from as usize] = Some((kind, old_promo, Owner::Sente));
                    push_ranked_parent(parent, ids);
                }
            }
        }
    }
}

fn push_ranked_parent(parent: Position, ids: &mut Vec<u32>) {
    if let Some(id) = rank_canonical(&parent) {
        ids.push(id);
    }
}

fn source_squares(child: &Position, to: u8, kind: Kind, promo: bool, owner: Owner) -> Vec<u8> {
    let (slider, dirs) = movement(kind, promo, owner);
    let tr = rank(to);
    let tf = file(to);
    let mut out = Vec::with_capacity(8);
    for (dr, df) in dirs {
        let mut r = tr - dr;
        let mut f = tf - df;
        while on_board(r, f) {
            let sq = (r * NFILE + f) as u8;
            if child.board[sq as usize].is_some() {
                break;
            }
            out.push(sq);
            if !slider {
                break;
            }
            r -= dr;
            f -= df;
        }
    }
    out
}

fn hand_index(kind: Kind) -> usize {
    match kind {
        Kind::Gold => 0,
        Kind::Silver => 1,
        Kind::Bishop => 2,
        Kind::Pawn => 3,
        Kind::King => panic!("king is never in hand"),
    }
}

fn rank(sq: u8) -> i8 {
    sq as i8 / NFILE
}

fn file(sq: u8) -> i8 {
    sq as i8 % NFILE
}

fn on_board(r: i8, f: i8) -> bool {
    r >= 0 && r < NRANK && f >= 0 && f < NFILE
}

fn gold_dirs(fwd: i8) -> [(i8, i8); 6] {
    [(fwd, -1), (fwd, 0), (fwd, 1), (0, -1), (0, 1), (-fwd, 0)]
}

fn movement(kind: Kind, promo: bool, owner: Owner) -> (bool, Vec<(i8, i8)>) {
    let f = match owner {
        Owner::Sente => -1,
        Owner::Gote => 1,
    };
    match (kind, promo) {
        (Kind::King, _) => (
            false,
            vec![
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, -1),
                (0, 1),
                (1, -1),
                (1, 0),
                (1, 1),
            ],
        ),
        (Kind::Gold, false) => (false, gold_dirs(f).to_vec()),
        (Kind::Gold, true) => (true, vec![(-1, 0), (1, 0), (0, -1), (0, 1)]),
        (Kind::Silver, false) => (false, vec![(f, -1), (f, 0), (f, 1), (-f, -1), (-f, 1)]),
        (Kind::Silver, true) => (true, vec![(f, 0)]),
        (Kind::Bishop, false) => (true, vec![(-1, -1), (-1, 1), (1, -1), (1, 1)]),
        (Kind::Bishop, true) => (false, gold_dirs(f).to_vec()),
        (Kind::Pawn, false) => (false, vec![(f, 0)]),
        (Kind::Pawn, true) => (false, vec![(2 * f, -1), (2 * f, 1)]),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet, VecDeque};

    use super::*;

    #[test]
    fn domain_size_matches_formula() {
        assert_eq!(kind_domain_size(18), 2_595);
        assert_eq!(per_king_size(), 5_361_993);
        assert_eq!(domain_size(), 2_037_557_340);
    }

    #[test]
    fn rank_roundtrips_samples() {
        let samples = [
            0,
            1,
            2,
            2_594,
            per_king_size() - 1,
            per_king_size(),
            domain_size() / 2,
            domain_size() - 1,
        ];
        for id in samples {
            let p = unrank_raw_sente(id);
            assert_eq!(rank_raw_sente(&p), Some(id));
        }
    }

    #[test]
    fn start_is_rankable_and_mirror_canonical() {
        let start = kpg_start();
        let id = rank_canonical(&start).unwrap();
        assert_eq!(rank_canonical(&mirror_sente(&start)), Some(id));
        assert_eq!(rank_raw_sente(&unrank_raw_sente(id)), Some(id));
    }

    #[test]
    fn predecessors_cover_depth_limited_forward_edges() {
        let start = kpg_start();
        let start_id = rank_canonical(&start).unwrap();
        let mut depth = HashMap::<u32, u8>::new();
        let mut q = VecDeque::new();
        depth.insert(start_id, 0);
        q.push_back(start_id);
        let mut edges = Vec::new();
        while let Some(pid) = q.pop_front() {
            let d = depth[&pid];
            if d == 3 {
                continue;
            }
            let p = unrank_raw_sente(pid);
            if p.is_immediate_win() {
                continue;
            }
            for m in p.moves() {
                if p.is_terminal_win_move(&m) {
                    continue;
                }
                let cid = rank_canonical(&p.make(&m)).unwrap();
                edges.push((pid, cid));
                if let std::collections::hash_map::Entry::Vacant(e) = depth.entry(cid) {
                    e.insert(d + 1);
                    q.push_back(cid);
                }
            }
        }
        assert!(!edges.is_empty());
        for (pid, cid) in edges {
            let preds = predecessor_ids_verified(cid);
            assert!(
                preds.ids.contains(&pid),
                "missing predecessor {pid} for child {cid}"
            );
        }
    }

    #[test]
    fn generated_predecessors_are_forward_valid_near_start() {
        let start = kpg_start();
        let start_id = rank_canonical(&start).unwrap();
        let mut children = HashSet::new();
        for m in start.moves() {
            if !start.is_terminal_win_move(&m) {
                children.insert(rank_canonical(&start.make(&m)).unwrap());
            }
        }
        for cid in children {
            let fast = predecessor_ids(cid);
            let verified = predecessor_ids_verified(cid);
            assert_eq!(fast.ids, verified.ids);
        }
        assert_eq!(rank_canonical(&start), Some(start_id));
    }

    #[test]
    fn single_orientation_predecessors_match_mirrored_generation() {
        let start = kpg_start();
        let start_id = rank_canonical(&start).unwrap();
        let mut seen = HashSet::new();
        let mut q = VecDeque::new();
        seen.insert(start_id);
        q.push_back(start_id);
        while seen.len() < 2_000 {
            let Some(pid) = q.pop_front() else {
                break;
            };
            let p = unrank_raw_sente(pid);
            if p.is_immediate_win() {
                continue;
            }
            for m in p.moves() {
                if p.is_terminal_win_move(&m) {
                    continue;
                }
                let cid = rank_canonical(&p.make(&m)).unwrap();
                if seen.insert(cid) {
                    q.push_back(cid);
                }
            }
        }
        for id in seen {
            let single = predecessor_ids(id);
            let mirrored = predecessor_ids_both_mirrors(id);
            assert_eq!(single.ids, mirrored.ids, "predecessors differ for {id}");
        }
    }
}
