# Micro Shogi Viewer

Standalone legal-move viewer for the Micro Shogi rules engine.

Open `index.html` directly in a browser. No build step or local server is required.

What it covers:

- 4x5 board and standard starting position.
- Gold, Silver, Bishop, Pawn, King movement.
- Capture-flip promotion.
- Unrestricted either-face drops.
- King-capture terminal handling.
- Move history, undo/redo, board rotation, and grouped legal moves.
- CSS-rendered board with standard `kanji_light` SVG shogi pieces from Lishogi.

What it does not cover yet:

- Tablebase values.
- Repetition adjudication beyond ordinary play-through.
- Import/export notation.

Piece art attribution:

- `kanji_light` shogi pieces by Ka-hu, via Lishogi.
- License: CC BY 4.0.
- Source: https://github.com/WandererXII/lishogi/tree/master/ui/@build/pieces/assets/standard/kanji_light
