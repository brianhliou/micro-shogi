(function () {
  "use strict";

  const rules = window.MicroShogi;
  const boardEl = document.getElementById("board");
  const topHandEl = document.getElementById("topHand");
  const bottomHandEl = document.getElementById("bottomHand");
  const statusTitleEl = document.getElementById("statusTitle");
  const statusSubtitleEl = document.getElementById("statusSubtitle");
  const movesEl = document.getElementById("moves");
  const historyEl = document.getElementById("history");
  const faceRowEl = document.getElementById("faceRow");
  const faceBaseEl = document.getElementById("faceBase");
  const facePromoEl = document.getElementById("facePromo");
  const resetEl = document.getElementById("reset");
  const undoEl = document.getElementById("undo");
  const redoEl = document.getElementById("redo");
  const flipEl = document.getElementById("flip");
  const rulesEl = document.getElementById("rules");
  const modalEl = document.getElementById("rulesModal");
  const closeModalEl = document.getElementById("closeModal");
  const PIECE_ASSET_FACE = {
    K: "OU",
    G: "KI",
    R: "HI",
    S: "GI",
    L: "KY",
    B: "KA",
    T: "TO",
    P: "FU",
    N: "KE",
  };

  let history = [rules.initialPosition()];
  let played = [];
  let cursor = 0;
  let selected = null;
  let dropPromo = false;
  let flipped = false;
  let dragState = null;
  let dragGhost = null;
  let hoverMove = null;

  function pos() {
    return history[cursor];
  }

  function moveKey(move) {
    return `${move.type}:${move.kind}:${move.from ?? "-"}:${move.to}:${move.promo ? 1 : 0}`;
  }

  function legal() {
    return rules.legalMoves(pos());
  }

  function pieceAsset(owner, face) {
    const prefix = owner === (flipped ? "G" : "S") ? "0" : "1";
    const code = face === "K" && owner === "G" ? "GY" : PIECE_ASSET_FACE[face];
    return `./assets/pieces/kanji-light/${prefix}${code}.svg`;
  }

  function selectedMoves() {
    if (!selected) return [];
    if (selected.type === "square") {
      return legal().filter((move) => move.from === selected.sq);
    }
    return legal().filter((move) => move.type === "drop" && move.kind === selected.kind && move.promo === dropPromo);
  }

  function draggedMove(sq) {
    if (!dragState) return null;
    if (dragState.type === "move") {
      return legal().find((move) => move.type === "move" && move.from === dragState.from && move.to === sq);
    }
    return legal().find((move) => (
      move.type === "drop" &&
      move.kind === dragState.kind &&
      move.promo === dragState.promo &&
      move.to === sq
    ));
  }

  function clearDragGhost() {
    if (!dragGhost) return;
    dragGhost.remove();
    dragGhost = null;
  }

  function setPieceDragImage(event, owner, face, sourceEl) {
    if (!event.dataTransfer) return;
    clearDragGhost();
    const rect = sourceEl?.getBoundingClientRect();
    const width = Math.max(1, Math.round(rect?.width || 76));
    const height = Math.max(1, Math.round(rect?.height || 76));
    const ghost = document.createElement("div");
    ghost.className = "drag-ghost";
    ghost.style.width = `${width}px`;
    ghost.style.height = `${height}px`;
    const img = document.createElement("img");
    img.src = pieceAsset(owner, face);
    img.alt = "";
    ghost.append(img);
    document.body.append(ghost);
    dragGhost = ghost;
    event.dataTransfer.setDragImage(ghost, Math.round(width / 2), Math.round(height / 2));
  }

  function setDropDragImage(event, owner, kind, sourceEl) {
    const face = dropPromo ? rules.PROMOTED_FACE[kind] : kind;
    setPieceDragImage(event, owner, face, sourceEl);
  }

  function play(move) {
    const current = pos();
    if (cursor < history.length - 1) {
      history = history.slice(0, cursor + 1);
      played = played.slice(0, cursor);
    }
    const label = rules.moveLabel(current, move);
    history.push(rules.makeMove(current, move));
    played.push({ move, label });
    cursor += 1;
    selected = null;
    hoverMove = null;
    render();
  }

  function ownerAtTop() {
    return flipped ? "S" : "G";
  }

  function ownerAtBottom() {
    return flipped ? "G" : "S";
  }

  function displaySquares() {
    const rows = flipped ? [4, 3, 2, 1, 0] : [0, 1, 2, 3, 4];
    const files = flipped ? [3, 2, 1, 0] : [0, 1, 2, 3];
    const out = [];
    for (const r of rows) {
      for (const f of files) {
        out.push(rules.square(r, f));
      }
    }
    return out;
  }

  function edgeLabels(sq) {
    const r = rules.rank(sq);
    const f = rules.file(sq);
    const topRank = flipped ? rules.NRANK - 1 : 0;
    const rightFile = flipped ? 0 : rules.NFILE - 1;
    const name = rules.squareName(sq);
    return {
      file: r === topRank ? name[0] : null,
      rank: f === rightFile ? name.slice(1) : null,
    };
  }

  function lastSquares() {
    if (cursor === 0) return new Set();
    const move = played[cursor - 1].move;
    const set = new Set([move.to]);
    if (move.from !== null && move.from !== undefined) set.add(move.from);
    return set;
  }

  function clearHoverMove() {
    hoverMove = null;
    renderMovePreview();
  }

  function setHoverMove(move) {
    hoverMove = move || null;
    renderMovePreview();
  }

  function displayPoint(sq) {
    const idx = displaySquares().indexOf(sq);
    if (idx < 0) return null;
    return {
      x: (idx % rules.NFILE + 0.5) * 100,
      y: (Math.floor(idx / rules.NFILE) + 0.5) * 100,
    };
  }

  function arrowPath(from, to) {
    const start = displayPoint(from);
    const end = displayPoint(to);
    if (!start || !end) return "";
    const dx = end.x - start.x;
    const dy = end.y - start.y;
    const len = Math.hypot(dx, dy);
    if (len < 1) return "";
    const ux = dx / len;
    const uy = dy / len;
    const px = -uy;
    const py = ux;
    const cell = 100;
    const shaftWidth = cell * 0.12;
    const headLength = cell * 0.32;
    const headHalfWidth = cell * 0.2;
    const sx = start.x + ux * cell * 0.22;
    const sy = start.y + uy * cell * 0.22;
    const tipX = end.x - ux * cell * 0.06;
    const tipY = end.y - uy * cell * 0.06;
    const baseX = tipX - ux * headLength;
    const baseY = tipY - uy * headLength;
    return [
      '<g class="move-preview-arrow">',
      `<line class="move-preview-line" x1="${sx.toFixed(2)}" y1="${sy.toFixed(2)}" x2="${baseX.toFixed(2)}" y2="${baseY.toFixed(2)}" stroke-width="${shaftWidth.toFixed(2)}"/>`,
      `<polygon class="move-preview-head" points="${tipX.toFixed(2)},${tipY.toFixed(2)} ${(baseX + px * headHalfWidth).toFixed(2)},${(baseY + py * headHalfWidth).toFixed(2)} ${(baseX - px * headHalfWidth).toFixed(2)},${(baseY - py * headHalfWidth).toFixed(2)}"/>`,
      "</g>",
    ].join("");
  }

  function renderMovePreview() {
    boardEl.querySelector(".move-preview")?.remove();
    if (!hoverMove) return;
    const to = displayPoint(hoverMove.to);
    if (!to) return;
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", "move-preview");
    svg.setAttribute("viewBox", `0 0 ${rules.NFILE * 100} ${rules.NRANK * 100}`);
    svg.setAttribute("aria-hidden", "true");
    svg.innerHTML = [
      hoverMove.type === "move" ? arrowPath(hoverMove.from, hoverMove.to) : "",
      hoverMove.type === "drop" ? `<circle class="move-preview-drop" cx="${to.x}" cy="${to.y}" r="28"/>` : "",
    ].join("");
    boardEl.append(svg);
  }

  function paintBoardHints() {
    const targets = new Map(selectedMoves().map((move) => [move.to, move]));
    for (const cell of boardEl.querySelectorAll(".cell")) {
      const sq = Number(cell.dataset.sq);
      cell.classList.remove("selected", "target", "drop-target", "capture-target");
      if (selected?.type === "square" && selected.sq === sq) cell.classList.add("selected");

      const targetMove = targets.get(sq);
      if (targetMove) {
        cell.classList.add("target");
        if (targetMove.type === "drop") cell.classList.add("drop-target");
        if (targetMove.capture) cell.classList.add("capture-target");
      }
    }
  }

  function renderBoard() {
    const current = pos();
    const targets = new Map(selectedMoves().map((move) => [move.to, move]));
    const last = lastSquares();
    boardEl.innerHTML = "";
    boardEl.classList.toggle("flipped", flipped);

    for (const sq of displaySquares()) {
      const cell = document.createElement("div");
      cell.className = "cell";
      cell.setAttribute("role", "button");
      cell.tabIndex = 0;
      cell.dataset.sq = String(sq);
      cell.title = rules.squareName(sq);
      if (last.has(sq)) cell.classList.add("last");
      if (selected?.type === "square" && selected.sq === sq) cell.classList.add("selected");

      const targetMove = targets.get(sq);
      if (targetMove) {
        cell.classList.add("target");
        if (targetMove.type === "drop") cell.classList.add("drop-target");
        if (targetMove.capture) cell.classList.add("capture-target");
      }

      const piece = current.board[sq];
      if (piece) {
        const pieceEl = document.createElement("div");
        pieceEl.className = `piece ${piece.owner === "G" ? "gote" : "sente"} ${piece.promo ? "promoted" : ""}`;
        const face = rules.faceOf(piece.kind, piece.promo);
        pieceEl.dataset.face = face;
        pieceEl.setAttribute("aria-label", `${rules.ownerName(piece.owner)} ${rules.FACE_NAME[face]}`);
        const img = document.createElement("img");
        img.className = "piece-img";
        img.src = pieceAsset(piece.owner, face);
        img.alt = "";
        img.draggable = false;
        pieceEl.append(img);
        pieceEl.title = `${rules.ownerName(piece.owner)} ${rules.FACE_NAME[face]}`;
        cell.append(pieceEl);

        if (!current.winner && piece.owner === current.turn) {
          pieceEl.draggable = true;
          pieceEl.classList.add("draggable-piece");
          pieceEl.addEventListener("dragstart", (event) => onCellDragStart(event, sq, piece));
          pieceEl.addEventListener("dragend", () => onCellDragEnd(sq));
        }
      }

      const labels = edgeLabels(sq);
      if (labels.file) {
        const file = document.createElement("span");
        file.className = "edge-label file-label";
        file.textContent = labels.file;
        cell.append(file);
      }
      if (labels.rank) {
        const rank = document.createElement("span");
        rank.className = "edge-label rank-label";
        rank.textContent = labels.rank;
        cell.append(rank);
      }
      cell.addEventListener("click", () => onCellClick(sq));
      cell.addEventListener("keydown", (event) => onCellKeyDown(event, sq));
      cell.addEventListener("pointerenter", () => onCellPointerEnter(sq));
      cell.addEventListener("pointerleave", () => onCellPointerLeave(sq));
      cell.addEventListener("dragover", (event) => onCellDragOver(event, sq));
      cell.addEventListener("drop", (event) => onCellDrop(event, sq));
      boardEl.append(cell);
    }
    renderMovePreview();
  }

  function onCellClick(sq) {
    const current = pos();
    if (current.winner) return;
    const target = selectedMoves().find((move) => move.to === sq);
    if (target) {
      play(target);
      return;
    }
    hoverMove = null;
    const cell = current.board[sq];
    if (cell && cell.owner === current.turn) {
      selected = { type: "square", sq };
    } else {
      selected = null;
    }
    render();
  }

  function onCellKeyDown(event, sq) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    onCellClick(sq);
  }

  function onCellPointerEnter(sq) {
    const target = selectedMoves().find((move) => move.to === sq);
    if (target) setHoverMove(target);
  }

  function onCellPointerLeave(sq) {
    if (hoverMove?.to === sq) clearHoverMove();
  }

  function onCellDragOver(event, sq) {
    if (!draggedMove(sq)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
  }

  function onCellDrop(event, sq) {
    const move = draggedMove(sq);
    if (!move) return;
    event.preventDefault();
    dragState = null;
    clearDragGhost();
    play(move);
  }

  function onCellDragStart(event, sq, piece) {
    if (!event.dataTransfer) return;
    hoverMove = null;
    selected = { type: "square", sq };
    dragState = { type: "move", from: sq };
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", rules.squareName(sq));
    setPieceDragImage(event, piece.owner, rules.faceOf(piece.kind, piece.promo), event.currentTarget);
    paintBoardHints();
    renderMoves();
  }

  function onCellDragEnd(sq) {
    clearDragGhost();
    dragState = null;
    if (selected?.type === "square" && selected.sq === sq) {
      selected = null;
      render();
    }
  }


  function renderHand(container, owner) {
    const current = pos();
    const active = !current.winner && current.turn === owner;
    container.classList.toggle("active", active);
    const pieces = container.querySelector(".hand-pieces");
    const label = container.querySelector(".hand-label");
    label.textContent = owner === "S" ? "Sente" : "Gote";
    pieces.innerHTML = "";

    for (const kind of rules.HAND_KINDS) {
      const count = current.hand[owner][kind];
      const chip = document.createElement("button");
      chip.type = "button";
      chip.className = `hand-chip ${owner === "G" ? "gote" : "sente"}`;
      chip.title = `${rules.ownerName(owner)} ${rules.FACE_NAME[kind]} in hand`;
      const img = document.createElement("img");
      img.className = "hand-img";
      img.src = pieceAsset(owner, kind);
      img.alt = "";
      img.draggable = false;
      chip.append(img);
      if (count <= 0 || !active) chip.classList.add("disabled");
      if (selected?.type === "hand" && selected.kind === kind && current.turn === owner) chip.classList.add("selected");
      chip.disabled = count <= 0 || !active;
      chip.draggable = count > 0 && active;
      chip.addEventListener("click", () => {
        hoverMove = null;
        selected = { type: "hand", kind };
        render();
      });
      chip.addEventListener("dragstart", (event) => {
        if (count <= 0 || !active || !event.dataTransfer) {
          event.preventDefault();
          return;
        }
        const promo = dropPromo;
        hoverMove = null;
        selected = { type: "hand", kind };
        dragState = { type: "drop", kind, promo };
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", `${kind}${promo ? "+" : ""}`);
        setDropDragImage(event, owner, kind, img);
        chip.classList.add("selected");
        paintBoardHints();
        renderMoves();
      });
      chip.addEventListener("dragend", () => {
        clearDragGhost();
        dragState = null;
        if (selected?.type === "hand" && selected.kind === kind) {
          selected = null;
          render();
        }
      });

      const badge = document.createElement("span");
      badge.className = "hand-count";
      badge.textContent = String(count);
      chip.append(badge);
      pieces.append(chip);
    }
  }

  function renderHands() {
    renderHand(topHandEl, ownerAtTop());
    renderHand(bottomHandEl, ownerAtBottom());
  }

  function renderStatus() {
    const current = pos();
    if (current.winner) {
      statusTitleEl.textContent = `${rules.ownerName(current.winner)} wins`;
      statusSubtitleEl.textContent = played[cursor - 1]?.label || "King captured";
      return;
    }
    statusTitleEl.textContent = `${rules.ownerName(current.turn)} to move`;
    statusSubtitleEl.textContent = rules.formatPosition(current);
  }

  function renderFaceControls() {
    if (selected?.type !== "hand") {
      faceRowEl.hidden = true;
      return;
    }
    faceRowEl.hidden = false;
    const promoFace = rules.PROMOTED_FACE[selected.kind];
    faceBaseEl.textContent = `${rules.FACE_GLYPH[selected.kind]} ${selected.kind}`;
    facePromoEl.textContent = `${rules.FACE_GLYPH[promoFace]} ${promoFace}`;
    faceBaseEl.classList.toggle("active", !dropPromo);
    facePromoEl.classList.toggle("active", dropPromo);
  }

  function tag(text, className) {
    const el = document.createElement("span");
    el.className = `tag ${className}`;
    el.textContent = text;
    return el;
  }

  function renderMoves() {
    const current = pos();
    const all = legal();
    movesEl.innerHTML = "";
    if (all.length === 0) {
      const empty = document.createElement("div");
      empty.className = "empty";
      empty.textContent = current.winner ? "Game over." : "No legal moves.";
      movesEl.append(empty);
      return;
    }

    const groups = [
      ["Captures", all.filter((move) => move.capture)],
      ["Board moves", all.filter((move) => move.type === "move" && !move.capture)],
      ["Drops", all.filter((move) => move.type === "drop")],
    ].filter(([, moves]) => moves.length > 0);

    const selectedSet = new Set(selectedMoves().map(moveKey));
    for (const [name, moves] of groups) {
      const header = document.createElement("div");
      header.className = "move-group";
      header.textContent = name;
      movesEl.append(header);

      for (const move of moves) {
        const row = document.createElement("button");
        row.type = "button";
        row.className = "move-row";
        if (selectedSet.has(moveKey(move))) row.style.background = "#f1ead9";

        const code = document.createElement("span");
        code.className = "move-code";
        code.textContent = rules.moveLabel(current, move);
        row.append(code);

        const tags = document.createElement("span");
        tags.className = "move-tags";
        if (move.type === "drop") tags.append(tag("drop", "drop"));
        else tags.append(tag("move", "move"));
        if (move.capture) tags.append(tag("capture", "capture"));
        if (move.type === "move" && current.board[move.from]?.promo !== move.promo) tags.append(tag("flip", "flip"));
        if (move.type === "drop" && move.promo) tags.append(tag("promoted", "flip"));
        row.append(tags);

        row.addEventListener("click", () => play(move));
        row.addEventListener("pointerenter", () => setHoverMove(move));
        row.addEventListener("pointerleave", () => {
          if (hoverMove === move) clearHoverMove();
        });
        movesEl.append(row);
      }
    }
  }

  function renderHistory() {
    historyEl.innerHTML = "";
    if (played.length === 0) {
      const empty = document.createElement("div");
      empty.className = "empty";
      empty.textContent = "Start position";
      historyEl.append(empty);
      return;
    }
    played.forEach((entry, index) => {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = `${index + 1}. ${entry.label}`;
      if (cursor === index + 1) button.classList.add("current");
      button.addEventListener("click", () => {
        cursor = index + 1;
        selected = null;
        hoverMove = null;
        render();
      });
      historyEl.append(button);
    });
  }

  function renderControls() {
    undoEl.disabled = cursor === 0;
    redoEl.disabled = cursor >= history.length - 1;
  }

  function render() {
    renderStatus();
    renderFaceControls();
    renderHands();
    renderBoard();
    renderHistory();
    renderMoves();
    renderControls();
  }

  resetEl.addEventListener("click", () => {
    history = [rules.initialPosition()];
    played = [];
    cursor = 0;
    selected = null;
    hoverMove = null;
    render();
  });

  undoEl.addEventListener("click", () => {
    if (cursor > 0) {
      cursor -= 1;
      selected = null;
      hoverMove = null;
      render();
    }
  });

  redoEl.addEventListener("click", () => {
    if (cursor < history.length - 1) {
      cursor += 1;
      selected = null;
      hoverMove = null;
      render();
    }
  });

  flipEl.addEventListener("click", () => {
    flipped = !flipped;
    hoverMove = null;
    render();
  });

  faceBaseEl.addEventListener("click", () => {
    dropPromo = false;
    render();
  });

  facePromoEl.addEventListener("click", () => {
    dropPromo = true;
    render();
  });

  rulesEl.addEventListener("click", () => {
    modalEl.hidden = false;
  });

  closeModalEl.addEventListener("click", () => {
    modalEl.hidden = true;
  });

  modalEl.addEventListener("click", (event) => {
    if (event.target === modalEl) modalEl.hidden = true;
  });

  document.addEventListener("pointerdown", (event) => {
    if (!selected || !(event.target instanceof Element)) return;
    if (event.target.closest("#board, .hand, #faceRow")) return;
    selected = null;
    hoverMove = null;
    render();
  }, true);

  window.addEventListener("dragend", () => {
    const wasDragging = Boolean(dragState);
    clearDragGhost();
    dragState = null;
    if (wasDragging && (selected?.type === "hand" || selected?.type === "square")) {
      selected = null;
      hoverMove = null;
      render();
    }
  });

  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      modalEl.hidden = true;
      selected = null;
      hoverMove = null;
      render();
    }
    if (event.key === "ArrowLeft" && cursor > 0) {
      cursor -= 1;
      selected = null;
      hoverMove = null;
      render();
    }
    if (event.key === "ArrowRight" && cursor < history.length - 1) {
      cursor += 1;
      selected = null;
      hoverMove = null;
      render();
    }
  });

  render();
})();
