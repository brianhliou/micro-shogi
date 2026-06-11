const assert = require("node:assert/strict");
const test = require("node:test");

const rules = require("./rules.js");

test("initial position round-trips and has nine legal moves", () => {
  const pos = rules.initialPosition();
  assert.equal(rules.formatPosition(pos), "S/kbgs/p---/----/---P/SGBK/-");
  assert.equal(rules.legalMoves(pos).length, 9);
});

test("perft matches committed engine output through depth 4", () => {
  const pos = rules.initialPosition();
  assert.equal(rules.perft(pos, 1), 9);
  assert.equal(rules.perft(pos, 2), 81);
  assert.equal(rules.perft(pos, 3), 812);
  assert.equal(rules.perft(pos, 4), 8512);
});

test("capture flips the moving piece and adds the captured base piece to hand", () => {
  const pos = rules.parsePosition("S/k---/g---/-S--/----/---K/-");
  const move = rules.legalMoves(pos).find((m) => m.from === rules.square(2, 1) && m.to === rules.square(1, 0));
  assert.ok(move);
  assert.equal(move.kind, "S");
  assert.equal(move.capture, true);
  assert.equal(move.promo, true);

  const next = rules.makeMove(pos, move);
  assert.deepEqual(next.board[rules.square(1, 0)], { kind: "S", promo: true, owner: "S" });
  assert.equal(next.hand.S.G, 1);
  assert.equal(next.turn, "G");
});

test("drops offer both faces on every empty square", () => {
  const pos = rules.parsePosition("S/k---/----/----/----/---K/S");
  const drops = rules.legalMoves(pos).filter((m) => m.type === "drop" && m.kind === "S");
  assert.equal(drops.filter((m) => !m.promo).length, 18);
  assert.equal(drops.filter((m) => m.promo).length, 18);
});

test("king capture is terminal", () => {
  const pos = rules.parsePosition("S/----/k---/-S--/----/---K/-");
  const move = rules.legalMoves(pos).find((m) => m.terminal);
  assert.ok(move);

  const next = rules.makeMove(pos, move);
  assert.equal(next.winner, "S");
  assert.equal(rules.legalMoves(next).length, 0);
});
