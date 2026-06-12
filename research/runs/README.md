# Calibration run outputs

This directory is for durable outputs from the reduced-piece calibration ladder.
The solver writes one text report and one JSON report per rung, and it writes them
twice: once immediately after the core solve, then again after validation. If a
large run is interrupted during validation, the files should still contain the
solved value with `validation_label = "pending"`.

## KPG rerun

Goal: recover the lost KPG result and turn it into a publishable ladder rung:
exact reachable count, start value, W/L/D split, max DTM, real branching, wall
time, validation status, and growth ratios over KP. This is a reduced-piece
subgame, not a partial solution of full Micro Shogi by itself.

Recommended: run this on a disposable cloud/bare-metal box, not on a daily-use
laptop or desktop. KPG is a long, silent, memory-heavy job; local interruption
cost is high.

Minimum practical machine for the current reachable-HashMap + stored-CSR driver:

- 96-128 GB RAM preferred; 64 GB is marginal and may swap or OOM
- 4+ CPU cores
- 20 GB free disk for source, build artifacts, logs, and the optional table dump
- Linux or macOS with Rust stable installed

Run from the repo root on a Linux cloud box:

```sh
RUN_ID=kpg-2026-06-11 research/runs/run-kpg-cloud.sh
```

On macOS, use BSD `time`:

```sh
mkdir -p research/runs/kpg-2026-06-11
/usr/bin/time -l cargo run --manifest-path solver/Cargo.toml --release --bin solve KPG \
  --out-dir research/runs/kpg-2026-06-11 \
  --dump-table \
  --audit-large 2>&1 | tee research/runs/kpg-2026-06-11/console.log
```

Observed cost from the successful 2026-06-11/12 Hetzner run:

- Core solve wall time: 12,385 s (3.44 h); end-to-end `/usr/bin/time` wall time
  including table dump and `--audit-large`: 4:21:08.
- Memory: the current driver stores the full reverse graph. The first KPG attempt
  exposed about 5.54B predecessor edges, so `par_idx` alone is about 20.6 GiB.
  With reachable keys, `HashMap`, values, counters, offsets, fill array, queue,
  audit, and allocator overhead, peak RSS was 63,486,272 KB (~60.5 GiB). A 64 GB
  box completed but was marginal and used swap during the run.
- The table dump is 17,385,741,380 bytes (869,287,068 records at 20 bytes/record,
  plus the 20-byte header).
- Dollars: effectively free locally; small on a rented high-memory machine, but
  verify current instance pricing before starting a paid cloud run.

The method tradeoff is documented in [`../solver-methods.md`](../solver-methods.md).
KPG currently uses a convenient in-RAM shortcut: reachable `HashMap` ids plus a
stored predecessor CSR. Shogi4's scalable method uses dense rank/unrank plus
on-demand unmove generation instead; that is the right production direction, but
it is a solver rewrite rather than a flag on this driver.

The `--audit-large` flag is intentionally included for the article run. Without
it, KPG will still write the value and counts, but the consistency audit will be
skipped because the table is over the default 5M-position validation gate.

Expected outputs:

- `research/runs/kpg-2026-06-11/kpg.txt`
- `research/runs/kpg-2026-06-11/kpg.json`
- `research/runs/kpg-2026-06-11/kpg.table.bin`
- `research/runs/kpg-2026-06-11/kpg.table.bin.zst`
- `research/runs/kpg-2026-06-11/console.log`
- `research/runs/kpg-2026-06-11/summary.md`
- `research/runs/kpg-2026-06-11/table-stats.txt`

Completed result:

```text
positions_reachable_canonical = 869287068
start_value = 0
verdict = draw
wins / losses / draws = 606922331 / 142074547 / 120290190
W / L / D percentages = 69.82% / 16.34% / 13.84%
max_dtm = 155
max_win_dtm = 155
max_loss_dtm = 154
mean_decided_dtm = 7.020263
p50/p90/p99/p99.9 decided DTM = 2 / 16 / 54 / 89
total_legal_moves = 10250756260
avg_branching = 11.792142
propagation_edges = 4567032875
propagation_ns_per_edge = 92.698
solve_wall_ns_per_legal_move = 1208.231
push_wall_secs = 12385.279855
audit_bad_positions = 0
push_vs_pull_mismatches = skipped
validation_label = PASS
table_dump_bytes = 17385741380
compressed_table_bytes = 2181879983
```

If the run is killed after the core solve but before audit finishes, keep the
JSON/TXT files. They are the result we lost last time. Rerun later with
`--audit-large` to replace `validation_label = "pending"` with `PASS` or `FAIL`.
If the run is killed during the table dump, the core result is still safe; rerun
with `--dump-table` to recreate only the local table artifact.

The table dump format is binary, little-endian:

- 8-byte magic: `MSHTB001`
- `u64` record count
- `u32` record size, currently 20
- repeated records: `u128` canonical key + `i32` value

The `.bin` and `.bin.zst` artifacts are intentionally ignored by git. Commit the
JSON/TXT/Markdown reports and keep the binary table local unless there is a
separate artifact-hosting plan.

Compress the table before transferring it off a cloud box:

```sh
zstd -T0 -6 -f -o research/runs/kpg-2026-06-11/kpg.table.bin.zst \
  research/runs/kpg-2026-06-11/kpg.table.bin
zstd -t research/runs/kpg-2026-06-11/kpg.table.bin.zst
sha256sum research/runs/kpg-2026-06-11/kpg.table.bin.zst
```

The KPG table compressed from 17,385,741,380 bytes to 2,181,879,983 bytes
(12.55%) in 0:20.74 on the Hetzner box. The verified compressed checksum is:

```text
2adf6533272b1430b4e6dd224009ab01af66a94361000aa92a18657e3fd0be1f  kpg.table.bin.zst
```

After dumping a table, scan it once for a signed value and DTM histogram:

```sh
cargo run --manifest-path solver/Cargo.toml --release --bin table_stats -- \
  research/runs/kpg-2026-06-11/kpg.table.bin \
  > research/runs/kpg-2026-06-11/table-stats.txt
```

The KPG scan took 0:25.87 on the Hetzner box and used about 55 MB RSS.

## Scope guardrail

KPG is the last in-RAM rung planned for this pass. Do not run KPGS, FULL, or
KPGSB for the article work. The driver exits unless `--allow-huge` is supplied:

```sh
cargo run --manifest-path solver/Cargo.toml --release --bin solve KPGS
```

That guard exists because KPGS is expected to be around 240x KPG and TB-scale in
RAM. `--allow-huge` is only an emergency override, not part of the current plan.

## Analysis checklist

KPG is complete. When drafting public prose from `kpg.json`:

- Add the exact KPG row to `research/findings.md` and the project `README.md`.
- Compute growth over KP:
  - `positions_reachable_canonical(KPG) / 457993`
  - `total_legal_moves(KPG) / 3143376`
  - `avg_branching(KPG) / 6.863371`
- Compute W/L/D percentages and draw fraction.
- Record `max_dtm` as the depth calibration for this rung.
- Record the `table_stats` DTM distribution; it confirms whether the reported
  max DTM is an absolute max or just a winning-position max.
- Use `solve_wall_ns_per_legal_move` for KPG-vs-KP wall-clock comparison.
- Keep `propagation_ns_per_edge` separate; it only times the final queue
  propagation phase of the push solver, not the full solve.
- Record `table_dump_bytes` as the exact persisted table size.
- Record audit result. Push-vs-pull cross-check is expected to be skipped for
  KPG because the independent pull solve is too expensive above 1M positions.
- In the website article, frame KPG as a "ladder rung" or "status of the solve,"
  not as "partially solving Micro Shogi" in the headline.

## Current KP baseline

Latest smoke run, from this driver:

```text
positions_reachable_canonical = 457993
start_value = 0
verdict = draw
wins / losses / draws = 135804 / 2956 / 319233
max_dtm = 29
total_legal_moves = 3143376
avg_branching = 6.863371
solve_wall_ns_per_legal_move ~= 354
audit_bad_positions = 0
push_vs_pull_mismatches = 0
validation_label = PASS
```
