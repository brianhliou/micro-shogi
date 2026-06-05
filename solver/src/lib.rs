//! Micro Shogi (4×5 drop-shogi) rules engine — from scratch.
//!
//! Rules per `research/rules.md`. Key differences from Dōbutsu: sliding pieces
//! (Bishop/Rook/Lance), promotion by **capture-flip** (mandatory, reversible, no
//! promotion zone), and **unrestricted** drops (no nifu / uchifuzume / last-rank
//! ban; a piece may be dropped on either face).
//!
//! Squares are 0..20 with `square = rank*4 + file`. Rank 0 is the top row (Gote's
//! back rank), rank 4 the bottom (Sente's back rank); files a,b,c,d are 0..3.
//! Sente starts at the bottom and advances toward rank 0 (forward = -rank).
//!
//! Each non-King piece has a base face and a reverse (promoted) face, with its own
//! letter: Gold↔Rook (G/R), Silver↔Lance (S/L), Bishop↔Tokin (B/T), Pawn↔Knight
//! (P/N), King (K, no reverse). Uppercase = Sente, lowercase = Gote. Position
//! string: `side / rank0 / rank1 / rank2 / rank3 / rank4 / hand`, e.g. the start
//! position `S/kbgs/p---/----/---P/SGBK/-`.
//!
//! Terminal = King capture (we use pseudo-legal move generation + king-capture as
//! the win, like the Dōbutsu solver; equivalent to checkmate under optimal play).

pub mod retro;

pub const NFILE: i8 = 4;
pub const NRANK: i8 = 5;
pub const NSQ: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Owner {
    Sente,
    Gote,
}
impl Owner {
    pub fn flip(self) -> Owner {
        match self {
            Owner::Sente => Owner::Gote,
            Owner::Gote => Owner::Sente,
        }
    }
    /// forward = drank toward the enemy back rank
    fn fwd(self) -> i8 {
        match self {
            Owner::Sente => -1,
            Owner::Gote => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Kind {
    King,
    Gold,
    Silver,
    Bishop,
    Pawn,
}
use Kind::*;

/// The four hand-holdable kinds, in hand-array order.
pub const HAND_KINDS: [Kind; 4] = [Gold, Silver, Bishop, Pawn];

impl Kind {
    fn hand_index(self) -> usize {
        match self {
            Gold => 0,
            Silver => 1,
            Bishop => 2,
            Pawn => 3,
            King => panic!("King is never held in hand"),
        }
    }
}

/// A board cell: (kind, promoted-face?, owner). King always has promoted = false.
type Cell = Option<(Kind, bool, Owner)>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Position {
    pub board: [Cell; NSQ],
    pub hand_sente: [u8; 4], // [Gold, Silver, Bishop, Pawn]
    pub hand_gote: [u8; 4],
    pub turn: Owner,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Move {
    pub kind: Kind,       // piece kind moved (or dropped)
    pub from: Option<u8>, // None = drop
    pub to: u8,
    pub capture: bool,
    pub promo: bool, // resulting FACE at `to` (board move: flipped on capture; drop: chosen face)
}

#[inline]
fn rank(sq: u8) -> i8 {
    (sq as i8) / NFILE
}
#[inline]
fn file(sq: u8) -> i8 {
    (sq as i8) % NFILE
}
#[inline]
fn on_board(r: i8, f: i8) -> bool {
    r >= 0 && r < NRANK && f >= 0 && f < NFILE
}

fn gold_dirs(fwd: i8) -> [(i8, i8); 6] {
    // orthogonals + the two forward diagonals (not the back diagonals)
    [
        (fwd, -1),
        (fwd, 0),
        (fwd, 1),
        (0, -1),
        (0, 1),
        (-fwd, 0),
    ]
}

/// (is_slider, directions). For sliders the dirs are ray directions; otherwise
/// single-step offsets (the Knight is a single jump, handled as a stepper).
fn movement(kind: Kind, promo: bool, owner: Owner) -> (bool, Vec<(i8, i8)>) {
    let f = owner.fwd();
    match (kind, promo) {
        (King, _) => (
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
        (Gold, false) => (false, gold_dirs(f).to_vec()),
        (Gold, true) => (true, vec![(-1, 0), (1, 0), (0, -1), (0, 1)]), // Rook
        (Silver, false) => (
            false,
            // forward + four diagonals
            vec![(f, -1), (f, 0), (f, 1), (-f, -1), (-f, 1)],
        ),
        (Silver, true) => (true, vec![(f, 0)]), // Lance (forward ray)
        (Bishop, false) => (true, vec![(-1, -1), (-1, 1), (1, -1), (1, 1)]), // Bishop (diagonal rays)
        (Bishop, true) => (false, gold_dirs(f).to_vec()),                   // Tokin = gold
        (Pawn, false) => (false, vec![(f, 0)]),
        (Pawn, true) => (false, vec![(2 * f, -1), (2 * f, 1)]), // Knight (jumps)
    }
}

impl Position {
    fn hand(&self, o: Owner) -> &[u8; 4] {
        match o {
            Owner::Sente => &self.hand_sente,
            Owner::Gote => &self.hand_gote,
        }
    }
    fn hand_mut(&mut self, o: Owner) -> &mut [u8; 4] {
        match o {
            Owner::Sente => &mut self.hand_sente,
            Owner::Gote => &mut self.hand_gote,
        }
    }

    /// All pseudo-legal moves for the side to move (board moves + drops).
    pub fn moves(&self) -> Vec<Move> {
        let mut out = Vec::with_capacity(40);
        self.board_moves(&mut out);
        self.drop_moves(&mut out);
        out
    }

    fn board_moves(&self, out: &mut Vec<Move>) {
        let me = self.turn;
        for sq in 0..NSQ as u8 {
            let (k, promo, o) = match self.board[sq as usize] {
                Some(x) => x,
                None => continue,
            };
            if o != me {
                continue;
            }
            let (r, f) = (rank(sq), file(sq));
            let (slider, dirs) = movement(k, promo, me);
            for (dr, df) in dirs {
                let mut nr = r + dr;
                let mut nf = f + df;
                loop {
                    if !on_board(nr, nf) {
                        break;
                    }
                    let nsq = (nr * NFILE + nf) as u8;
                    match self.board[nsq as usize] {
                        Some((_, _, o2)) if o2 == me => break, // own piece blocks
                        cell => {
                            let capture = cell.is_some();
                            // capture-flip: any capture flips the mover's face
                            // (King has no reverse and never flips).
                            let result_promo = if capture && k != King { !promo } else { promo };
                            out.push(Move {
                                kind: k,
                                from: Some(sq),
                                to: nsq,
                                capture,
                                promo: result_promo,
                            });
                            if capture || !slider {
                                break; // capture ends a ray; steppers do one step
                            }
                        }
                    }
                    nr += dr;
                    nf += df;
                }
            }
        }
    }

    fn drop_moves(&self, out: &mut Vec<Move>) {
        let me = self.turn;
        for (i, &k) in HAND_KINDS.iter().enumerate() {
            if self.hand(me)[i] == 0 {
                continue;
            }
            for nsq in 0..NSQ as u8 {
                if self.board[nsq as usize].is_some() {
                    continue;
                }
                // unrestricted drops: either face is legal
                for &promo in &[false, true] {
                    out.push(Move {
                        kind: k,
                        from: None,
                        to: nsq,
                        capture: false,
                        promo,
                    });
                }
            }
        }
    }

    /// Apply a move, returning the resulting position (turn flipped). A captured
    /// piece enters the captor's hand as its base kind (face-agnostic). This must
    /// NOT be called on a king-capturing move (that is terminal — see
    /// `is_terminal_win_move`).
    pub fn make(&self, mv: &Move) -> Position {
        let mut p = *self;
        let me = self.turn;
        match mv.from {
            Some(sq) => {
                p.board[sq as usize] = None;
                if mv.capture {
                    if let Some((ck, _, _)) = self.board[mv.to as usize] {
                        debug_assert!(ck != King, "king capture is terminal, not a make()");
                        p.hand_mut(me)[ck.hand_index()] += 1;
                    }
                }
                p.board[mv.to as usize] = Some((mv.kind, mv.promo, me));
            }
            None => {
                p.hand_mut(me)[mv.kind.hand_index()] -= 1;
                p.board[mv.to as usize] = Some((mv.kind, mv.promo, me));
            }
        }
        p.turn = me.flip();
        p
    }

    /// Does this move win on the spot by capturing the enemy King? Such moves end
    /// the game and have no successor position.
    pub fn is_terminal_win_move(&self, m: &Move) -> bool {
        m.capture && matches!(self.board[m.to as usize], Some((King, _, _)))
    }

    /// The side to move can capture the enemy King immediately.
    pub fn is_immediate_win(&self) -> bool {
        self.moves().iter().any(|m| self.is_terminal_win_move(m))
    }
}

// ---- (de)serialization ----

fn cell_char(k: Kind, promo: bool) -> char {
    match (k, promo) {
        (King, _) => 'K',
        (Gold, false) => 'G',
        (Gold, true) => 'R',
        (Silver, false) => 'S',
        (Silver, true) => 'L',
        (Bishop, false) => 'B',
        (Bishop, true) => 'T',
        (Pawn, false) => 'P',
        (Pawn, true) => 'N',
    }
}

fn char_cell(c: char) -> Option<(Kind, bool, Owner)> {
    let owner = if c.is_ascii_uppercase() {
        Owner::Sente
    } else {
        Owner::Gote
    };
    let (k, promo) = match c.to_ascii_uppercase() {
        'K' => (King, false),
        'G' => (Gold, false),
        'R' => (Gold, true),
        'S' => (Silver, false),
        'L' => (Silver, true),
        'B' => (Bishop, false),
        'T' => (Bishop, true),
        'P' => (Pawn, false),
        'N' => (Pawn, true),
        _ => return None,
    };
    Some((k, promo, owner))
}

pub fn parse(s: &str) -> Option<Position> {
    let parts: Vec<&str> = s.trim().split('/').collect();
    if parts.len() != 7 {
        return None;
    }
    let turn = match parts[0] {
        "S" => Owner::Sente,
        "G" => Owner::Gote,
        _ => return None,
    };
    let mut board: [Cell; NSQ] = [None; NSQ];
    for r in 0..NRANK as usize {
        let row: Vec<char> = parts[1 + r].chars().collect();
        if row.len() != NFILE as usize {
            return None;
        }
        for f in 0..NFILE as usize {
            if row[f] != '-' {
                board[r * NFILE as usize + f] = Some(char_cell(row[f])?);
            }
        }
    }
    let mut hand_sente = [0u8; 4];
    let mut hand_gote = [0u8; 4];
    if parts[6] != "-" {
        for c in parts[6].chars() {
            let (k, _, o) = char_cell(c)?;
            match o {
                Owner::Sente => hand_sente[k.hand_index()] += 1,
                Owner::Gote => hand_gote[k.hand_index()] += 1,
            }
        }
    }
    Some(Position {
        board,
        hand_sente,
        hand_gote,
        turn,
    })
}

pub fn format(p: &Position) -> String {
    let mut s = String::new();
    s.push(if p.turn == Owner::Sente { 'S' } else { 'G' });
    for r in 0..NRANK as usize {
        s.push('/');
        for f in 0..NFILE as usize {
            match p.board[r * NFILE as usize + f] {
                None => s.push('-'),
                Some((k, promo, o)) => {
                    let ch = cell_char(k, promo);
                    s.push(if o == Owner::Sente {
                        ch
                    } else {
                        ch.to_ascii_lowercase()
                    });
                }
            }
        }
    }
    s.push('/');
    let mut hand = String::new();
    for (i, &k) in HAND_KINDS.iter().enumerate() {
        for _ in 0..p.hand_sente[i] {
            hand.push(cell_char(k, false));
        }
    }
    for (i, &k) in HAND_KINDS.iter().enumerate() {
        for _ in 0..p.hand_gote[i] {
            hand.push(cell_char(k, false).to_ascii_lowercase());
        }
    }
    if hand.is_empty() {
        hand.push('-');
    }
    s.push_str(&hand);
    s
}

// ---- packing (u128 key) ----
// 20 cells × 5 bits (codes 0..18) = 100 bits; 8 hand counts × 2 bits = 16 bits;
// turn 1 bit. 117 bits total, fits in u128.

fn cell_code(c: Cell) -> u32 {
    match c {
        None => 0,
        Some((k, promo, o)) => {
            let base: u32 = match (k, promo) {
                (King, _) => 0,
                (Gold, false) => 1,
                (Gold, true) => 2,
                (Silver, false) => 3,
                (Silver, true) => 4,
                (Bishop, false) => 5,
                (Bishop, true) => 6,
                (Pawn, false) => 7,
                (Pawn, true) => 8,
            };
            base * 2 + if o == Owner::Gote { 1 } else { 0 } + 1
        }
    }
}

fn code_cell(code: u32) -> Cell {
    if code == 0 {
        return None;
    }
    let owner = if (code - 1) % 2 == 1 {
        Owner::Gote
    } else {
        Owner::Sente
    };
    let base = (code - 1) / 2;
    let (k, promo) = match base {
        0 => (King, false),
        1 => (Gold, false),
        2 => (Gold, true),
        3 => (Silver, false),
        4 => (Silver, true),
        5 => (Bishop, false),
        6 => (Bishop, true),
        7 => (Pawn, false),
        _ => (Pawn, true),
    };
    Some((k, promo, owner))
}

pub fn pack(p: &Position) -> u128 {
    let mut x: u128 = 0;
    for sq in 0..NSQ {
        x |= (cell_code(p.board[sq]) as u128) << (sq * 5);
    }
    let mut bit = 100;
    for &c in p.hand_sente.iter().chain(p.hand_gote.iter()) {
        x |= (c as u128) << bit;
        bit += 2;
    }
    if p.turn == Owner::Gote {
        x |= 1u128 << 116;
    }
    x
}

pub fn unpack(x: u128) -> Position {
    let mut board: [Cell; NSQ] = [None; NSQ];
    for sq in 0..NSQ {
        let code = ((x >> (sq * 5)) & 0x1F) as u32;
        board[sq] = code_cell(code);
    }
    let mut h = [0u8; 8];
    for (i, slot) in h.iter_mut().enumerate() {
        *slot = ((x >> (100 + 2 * i)) & 0x3) as u8;
    }
    let turn = if (x >> 116) & 1 == 1 {
        Owner::Gote
    } else {
        Owner::Sente
    };
    Position {
        board,
        hand_sente: [h[0], h[1], h[2], h[3]],
        hand_gote: [h[4], h[5], h[6], h[7]],
        turn,
    }
}

// ---- symmetry / canonical key ----

/// Turn-normalize: a Gote-to-move position is rotated 180° with colours swapped
/// (value-equivalent), giving an always-Sente-to-move representative.
pub fn canonical_turn(p: &Position) -> Position {
    if p.turn == Owner::Sente {
        return *p;
    }
    let mut board: [Cell; NSQ] = [None; NSQ];
    for s in 0..NSQ {
        let r = s as i8 / NFILE;
        let f = s as i8 % NFILE;
        let s2 = (((NRANK - 1 - r) * NFILE) + (NFILE - 1 - f)) as usize;
        board[s2] = p.board[s].map(|(k, promo, o)| (k, promo, o.flip()));
    }
    Position {
        board,
        hand_sente: p.hand_gote,
        hand_gote: p.hand_sente,
        turn: Owner::Sente,
    }
}

/// Left-right file mirror (f ↔ NFILE-1-f): a value-preserving spatial symmetry
/// (all piece moves are LR-symmetric).
fn mirror(p: &Position) -> Position {
    let mut board: [Cell; NSQ] = [None; NSQ];
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

/// Canonical packed key, folding turn (180° + colour swap) and LR mirror.
pub fn canonical_key(p: &Position) -> u128 {
    let q = canonical_turn(p);
    pack(&q).min(pack(&mirror(&q)))
}

/// The standard starting position.
pub fn initial() -> Position {
    parse("S/kbgs/p---/----/---P/SGBK/-").expect("valid init string")
}

/// Count of legal move sequences to a given depth (king-capture ends a line).
pub fn perft(p: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut n = 0;
    for m in p.moves() {
        if p.is_terminal_win_move(&m) {
            n += 1; // king capture: a leaf, no recursion
        } else {
            n += perft(&p.make(&m), depth - 1);
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    const INIT: &str = "S/kbgs/p---/----/---P/SGBK/-";

    #[test]
    fn init_roundtrips() {
        assert_eq!(format(&initial()), INIT);
        assert_eq!(format(&parse(INIT).unwrap()), INIT);
    }

    #[test]
    fn pack_roundtrips() {
        let p = initial();
        assert_eq!(unpack(pack(&p)), p);
        // promoted pieces (R/L/T/N) + pieces in hand + Gote to move
        let q = parse("G/k-R-/T---/--N-/L---/---K/GSbp").unwrap();
        assert_eq!(unpack(pack(&q)), q);
        assert_ne!(pack(&p), pack(&q));
    }

    #[test]
    fn initial_move_count_is_nine() {
        // Hand-derived: Silver 2 + Gold 3 + Bishop 2 + King 1 + Pawn 1 = 9.
        let p = initial();
        let moves = p.moves();
        assert_eq!(moves.len(), 9, "moves: {:?}", moves);
        assert!(moves.iter().all(|m| !m.capture));
        assert!(moves.iter().all(|m| m.from.is_some())); // no drops at start
    }

    #[test]
    fn symmetry_preserves_move_count_and_key() {
        let p = initial();
        // LR mirror and the turn-swapped form must have the same number of moves
        // and the same canonical key as the start.
        let m = mirror(&p);
        assert_eq!(p.moves().len(), m.moves().len());
        assert_eq!(canonical_key(&p), canonical_key(&m));
        // perft depth 2 is mirror-invariant
        assert_eq!(perft(&p, 2), perft(&m, 2));
    }

    #[test]
    fn canonical_turn_normalizes_to_sente() {
        let g = parse("G/kbgs/p---/----/---P/SGBK/-").unwrap();
        let c = canonical_turn(&g);
        assert_eq!(c.turn, Owner::Sente);
        // turn-normalizing a Sente position is identity
        let s = initial();
        assert_eq!(canonical_turn(&s), s);
    }

    #[test]
    fn capture_flips_face_and_fills_hand() {
        // Sente silver at c3 (rank2,file1) captures Gote gold at a4 (rank1,file0)
        // diagonally forward-left, flipping to a Lance; the gold enters Sente's hand.
        let p = parse("S/k---/g---/-S--/----/---K/-").unwrap();
        let mv = p
            .moves()
            .into_iter()
            .find(|m| m.from == Some(2 * 4 + 1) && m.to == 4 && m.capture)
            .expect("silver capture exists");
        assert_eq!(mv.kind, Silver);
        assert!(mv.promo, "capture must flip Silver to Lance");
        let q = p.make(&mv);
        assert_eq!(q.board[4], Some((Silver, true, Owner::Sente)));
        assert_eq!(q.board[2 * 4 + 1], None);
        assert_eq!(q.hand_sente[Gold.hand_index()], 1);
        assert_eq!(q.turn, Owner::Gote);
    }

    #[test]
    fn drops_offer_both_faces() {
        // Sente to move with a Silver in hand.
        let p = parse("S/k---/----/----/----/---K/S").unwrap();
        let moves = p.moves();
        let base = moves.iter().filter(|m| m.from.is_none() && !m.promo).count();
        let promoted = moves.iter().filter(|m| m.from.is_none() && m.promo).count();
        // 18 empty squares, each droppable in either face
        assert_eq!(base, 18);
        assert_eq!(promoted, 18);
    }

    #[test]
    fn king_capture_is_terminal() {
        // Sente silver at c3 can capture the Gote king at a4.
        let p = parse("S/----/k---/-S--/----/---K/-").unwrap();
        assert!(p.is_immediate_win());
        let mv = p
            .moves()
            .into_iter()
            .find(|m| p.is_terminal_win_move(m))
            .unwrap();
        assert_eq!(mv.to, 4);
        assert!(mv.capture);
    }
}
