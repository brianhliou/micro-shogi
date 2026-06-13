# micro-shogi

Rules engine, tablebase experiments, and run records for **Micro Shogi**, a
4x5 drop-shogi variant with five pieces per side.

This repository supports the public writeup:
[Toward Solving Micro Shogi](https://brianhliou.com/posts/micro-shogi/).
The full game is not solved. The current result is a measured calibration rung:
King+Pawn+Gold per side is solved and audited, and it was large enough to test
the solver architecture that a full solve would need.

## Current results

| Result | Value |
|---|---:|
| Micro Shogi all-arrangements upper bound | 3,915,109,365,634,620 |
| Full-game reachable positions | ~3.0e14 to ~6.2e14, estimated |
| Full-game dense-rank working table | ~1 PB for W/L/D values |
| KP reduced game | draw, 457,993 canonical reachable positions |
| KPG reduced game | draw, 869,287,068 canonical reachable positions |
| KPG max DTM | 155 plies |

The all-arrangements count is exact. The reachable count is still an estimate,
bracketed from Dōbutsu Shōgi and Minishogi ratios.

The KPG run produced two useful engineering measurements:

| Method | Peak memory | Time | Notes |
|---|---:|---:|---|
| Reachable ids + stored CSR | ~60.5 GiB | 4:21:08 full run | Audited |
| Dense rank + generated predecessors | ~6.86 GiB | 2:21:04 total | Matched audited values |

The dense-rank result is the main architectural lesson. A small solver can index
only reachable positions and store every reverse edge. A scalable solver needs a
mathematical rank over an arrangement domain, flat arrays, and predecessor
generation on demand.

## Where this fits

Micro Shogi sits above two earlier small drop-shogi projects:

- [Dōbutsu Shōgi](https://brianhliou.com/posts/dobutsu-shogi/) is fully solved:
  246,803,167 reachable positions.
- [Shogi4](https://brianhliou.com/posts/shogi4/) is not fully solved, but it has
  a validated dense-rank solver design and a 2,100,849,024-position closed run.

Micro Shogi is larger than Shogi4 as a full game, but the KPG calibration rung is
smaller than the Shogi4 closed run because it removes Silver and Bishop. KPG is
still important because it is where the convenient Micro Shogi solver hit the
memory wall and where the Shogi4-style dense-rank method proved itself on Micro
Shogi data.

## Repository layout

```text
research/
  findings.md        verified facts ledger and current estimates
  solver-methods.md  reachable-CSR versus dense-rank/unmove tradeoffs
  rules.md           ruleset implemented by the solver
  cost-model.md      full-solve sizing and cost estimates
  architecture.md    distributed strong-solve design notes
  runs/              committed run summaries, logs, and JSON reports
  repro/             state-space upper-bound reproduction scripts
solver/
  src/lib.rs         Rust rules engine
  src/retro.rs       pull and push retrograde calibration solvers
  src/dense_kpg.rs   KPG dense-rank comparison solver
viewer/
  index.html         standalone legal-move viewer
  rules.js           JavaScript rules engine used by the viewer
```

Large binary table artifacts are intentionally ignored. The KPG table dump is
17.4 GB raw and 2.18 GB compressed; this repo stores the small reports and
checksums, not the table itself.

## Useful commands

Run the solver checks:

```sh
cargo test --manifest-path solver/Cargo.toml
cargo check --manifest-path solver/Cargo.toml --bin solve_dense_kpg
```

Run the dense KPG solver on a machine with enough time and memory:

```sh
cargo run --manifest-path solver/Cargo.toml --release --bin solve_dense_kpg -- \
  --out-dir research/runs/dense-kpg-local
```

Open `viewer/index.html` in a browser to use the standalone legal-move viewer.

## Open work

- Confirm any remaining Micro Shogi rules gaps against a primary source or an
  independent engine.
- Replace the full-game reachable-count estimate with an exact enumeration.
- Generalize dense rank/unrank and predecessor generation beyond KPG.
- Run a distributed rehearsal before attempting a full Micro Shogi tablebase.
