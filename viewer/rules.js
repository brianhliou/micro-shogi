(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) {
    module.exports = api;
  }
  root.MicroShogi = api;
})(typeof globalThis !== "undefined" ? globalThis : this, function () {
  "use strict";

  const NFILE = 4;
  const NRANK = 5;
  const NSQ = NFILE * NRANK;
  const FILES = ["a", "b", "c", "d"];
  const HAND_KINDS = ["G", "S", "B", "P"];
  const PROMOTED_FACE = { G: "R", S: "L", B: "T", P: "N" };
  const FACE_KIND = { K: "K", G: "G", R: "G", S: "S", L: "S", B: "B", T: "B", P: "P", N: "P" };
  const FACE_PROMO = { K: false, G: false, R: true, S: false, L: true, B: false, T: true, P: false, N: true };
  const FACE_NAME = {
    K: "King",
    G: "Gold",
    R: "Rook",
    S: "Silver",
    L: "Lance",
    B: "Bishop",
    T: "Tokin",
    P: "Pawn",
    N: "Knight",
  };
  const FACE_GLYPH = {
    K: "玉",
    G: "金",
    R: "飛",
    S: "銀",
    L: "香",
    B: "角",
    T: "と",
    P: "歩",
    N: "桂",
  };

  function opponent(owner) {
    return owner === "S" ? "G" : "S";
  }

  function ownerName(owner) {
    return owner === "S" ? "Sente" : "Gote";
  }

  function forward(owner) {
    return owner === "S" ? -1 : 1;
  }

  function rank(sq) {
    return Math.floor(sq / NFILE);
  }

  function file(sq) {
    return sq % NFILE;
  }

  function square(r, f) {
    return r * NFILE + f;
  }

  function onBoard(r, f) {
    return r >= 0 && r < NRANK && f >= 0 && f < NFILE;
  }

  function squareName(sq) {
    return `${FILES[file(sq)]}${NRANK - rank(sq)}`;
  }

  function parseSquare(name) {
    if (!/^[a-d][1-5]$/.test(name)) return null;
    const f = FILES.indexOf(name[0]);
    const r = NRANK - Number(name[1]);
    return square(r, f);
  }

  function goldDirs(fwd) {
    return [
      [fwd, -1],
      [fwd, 0],
      [fwd, 1],
      [0, -1],
      [0, 1],
      [-fwd, 0],
    ];
  }

  function movement(kind, promo, owner) {
    const f = forward(owner);
    if (kind === "K") {
      return {
        slider: false,
        dirs: [
          [-1, -1],
          [-1, 0],
          [-1, 1],
          [0, -1],
          [0, 1],
          [1, -1],
          [1, 0],
          [1, 1],
        ],
      };
    }
    if (kind === "G" && !promo) return { slider: false, dirs: goldDirs(f) };
    if (kind === "G" && promo) return { slider: true, dirs: [[-1, 0], [1, 0], [0, -1], [0, 1]] };
    if (kind === "S" && !promo) return { slider: false, dirs: [[f, -1], [f, 0], [f, 1], [-f, -1], [-f, 1]] };
    if (kind === "S" && promo) return { slider: true, dirs: [[f, 0]] };
    if (kind === "B" && !promo) return { slider: true, dirs: [[-1, -1], [-1, 1], [1, -1], [1, 1]] };
    if (kind === "B" && promo) return { slider: false, dirs: goldDirs(f) };
    if (kind === "P" && !promo) return { slider: false, dirs: [[f, 0]] };
    return { slider: false, dirs: [[2 * f, -1], [2 * f, 1]] };
  }

  function emptyHands() {
    return {
      S: { G: 0, S: 0, B: 0, P: 0 },
      G: { G: 0, S: 0, B: 0, P: 0 },
    };
  }

  function clonePosition(pos) {
    return {
      board: pos.board.map((cell) => (cell ? { kind: cell.kind, promo: cell.promo, owner: cell.owner } : null)),
      hand: {
        S: { ...pos.hand.S },
        G: { ...pos.hand.G },
      },
      turn: pos.turn,
      winner: pos.winner || null,
    };
  }

  function charToCell(ch) {
    const upper = ch.toUpperCase();
    const kind = FACE_KIND[upper];
    if (!kind) return null;
    return {
      kind,
      promo: FACE_PROMO[upper],
      owner: ch === upper ? "S" : "G",
    };
  }

  function faceOf(kind, promo) {
    if (kind === "K") return "K";
    return promo ? PROMOTED_FACE[kind] : kind;
  }

  function cellToChar(cell) {
    const face = faceOf(cell.kind, cell.promo);
    return cell.owner === "S" ? face : face.toLowerCase();
  }

  function parsePosition(text) {
    const parts = text.trim().split("/");
    if (parts.length !== 7) return null;
    if (parts[0] !== "S" && parts[0] !== "G") return null;
    const board = new Array(NSQ).fill(null);
    for (let r = 0; r < NRANK; r += 1) {
      if (parts[r + 1].length !== NFILE) return null;
      for (let f = 0; f < NFILE; f += 1) {
        const ch = parts[r + 1][f];
        if (ch !== "-") {
          const cell = charToCell(ch);
          if (!cell) return null;
          board[square(r, f)] = cell;
        }
      }
    }
    const hand = emptyHands();
    if (parts[6] !== "-") {
      for (const ch of parts[6]) {
        const cell = charToCell(ch);
        if (!cell || cell.kind === "K") return null;
        hand[cell.owner][cell.kind] += 1;
      }
    }
    return { board, hand, turn: parts[0], winner: null };
  }

  function formatPosition(pos) {
    const rows = [pos.turn];
    for (let r = 0; r < NRANK; r += 1) {
      let row = "";
      for (let f = 0; f < NFILE; f += 1) {
        const cell = pos.board[square(r, f)];
        row += cell ? cellToChar(cell) : "-";
      }
      rows.push(row);
    }
    let hand = "";
    for (const kind of HAND_KINDS) {
      hand += kind.repeat(pos.hand.S[kind]);
    }
    for (const kind of HAND_KINDS) {
      hand += kind.toLowerCase().repeat(pos.hand.G[kind]);
    }
    rows.push(hand || "-");
    return rows.join("/");
  }

  function initialPosition() {
    return parsePosition("S/kbgs/p---/----/---P/SGBK/-");
  }

  function legalMoves(pos) {
    if (pos.winner) return [];
    const out = [];
    boardMoves(pos, out);
    dropMoves(pos, out);
    return out;
  }

  function boardMoves(pos, out) {
    const me = pos.turn;
    for (let from = 0; from < NSQ; from += 1) {
      const cell = pos.board[from];
      if (!cell || cell.owner !== me) continue;
      const move = movement(cell.kind, cell.promo, me);
      for (const [dr, df] of move.dirs) {
        let r = rank(from) + dr;
        let f = file(from) + df;
        while (onBoard(r, f)) {
          const to = square(r, f);
          const target = pos.board[to];
          if (target && target.owner === me) break;
          const capture = Boolean(target);
          out.push({
            type: "move",
            kind: cell.kind,
            from,
            to,
            capture,
            promo: capture && cell.kind !== "K" ? !cell.promo : cell.promo,
            terminal: capture && target.kind === "K",
          });
          if (capture || !move.slider) break;
          r += dr;
          f += df;
        }
      }
    }
  }

  function dropMoves(pos, out) {
    const me = pos.turn;
    for (const kind of HAND_KINDS) {
      if (pos.hand[me][kind] <= 0) continue;
      for (let to = 0; to < NSQ; to += 1) {
        if (pos.board[to]) continue;
        out.push({ type: "drop", kind, from: null, to, capture: false, promo: false, terminal: false });
        out.push({ type: "drop", kind, from: null, to, capture: false, promo: true, terminal: false });
      }
    }
  }

  function makeMove(pos, move) {
    const next = clonePosition(pos);
    const me = pos.turn;
    if (move.type === "drop") {
      next.hand[me][move.kind] -= 1;
      next.board[move.to] = { kind: move.kind, promo: move.promo, owner: me };
    } else {
      next.board[move.from] = null;
      const captured = pos.board[move.to];
      if (captured && captured.kind !== "K") {
        next.hand[me][captured.kind] += 1;
      }
      next.board[move.to] = { kind: move.kind, promo: move.promo, owner: me };
    }
    if (move.terminal) {
      next.winner = me;
    } else {
      next.turn = opponent(me);
    }
    return next;
  }

  function movesFromSquare(pos, sq) {
    return legalMoves(pos).filter((move) => move.from === sq);
  }

  function dropsFor(pos, kind, promo) {
    return legalMoves(pos).filter((move) => move.type === "drop" && move.kind === kind && move.promo === promo);
  }

  function moveLabel(pos, move) {
    const face = faceOf(move.kind, move.promo);
    if (move.type === "drop") {
      return `${face}*${squareName(move.to)}`;
    }
    const before = pos.board[move.from];
    const startFace = faceOf(before.kind, before.promo);
    const sep = move.capture ? "x" : "-";
    const flip = face !== startFace ? `=${face}` : "";
    const mate = move.terminal ? "#" : "";
    return `${startFace}${squareName(move.from)}${sep}${squareName(move.to)}${flip}${mate}`;
  }

  function perft(pos, depth) {
    if (depth === 0) return 1;
    let count = 0;
    for (const move of legalMoves(pos)) {
      if (move.terminal) {
        count += 1;
      } else {
        count += perft(makeMove(pos, move), depth - 1);
      }
    }
    return count;
  }

  return {
    NSQ,
    NFILE,
    NRANK,
    HAND_KINDS,
    PROMOTED_FACE,
    FACE_NAME,
    FACE_GLYPH,
    clonePosition,
    faceOf,
    file,
    formatPosition,
    initialPosition,
    legalMoves,
    makeMove,
    moveLabel,
    movesFromSquare,
    opponent,
    ownerName,
    parsePosition,
    parseSquare,
    perft,
    rank,
    square,
    squareName,
    dropsFor,
  };
});
