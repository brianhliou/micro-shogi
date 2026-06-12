# KPG run summary — 2026-06-11/12

Reduced-piece Micro Shogi rung: K + P + G per side.

Command:

```sh
RUN_ID=kpg-2026-06-11 research/runs/run-kpg-cloud.sh
```

Remote host:

- Provider / model: Hetzner vServer, KVM
- Hostname: `ubuntu-64gb-fsn1-1`
- OS: Ubuntu 26.04 LTS
- Kernel: Linux 7.0.0-15-generic
- CPU: 16 vCPU, AMD EPYC-Milan Processor, 8 cores / 16 threads, 32 MiB L3
- RAM: 61 GiB
- Swap: 64 GiB `/swapfile`
- Disk: 343 GiB root disk, 338 GiB filesystem
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `cargo 1.96.0 (30a34c682 2026-05-25)`

Important source note: `/root/micro-shogi` on the remote was a source copy, not a git checkout.
The successful rerun used the local CSR-offset fix copied to
`/root/micro-shogi/solver/src/retro.rs`.

## Result

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
p50_decided_dtm = 2
p90_decided_dtm = 16
p99_decided_dtm = 54
p999_decided_dtm = 89
total_legal_moves = 10250756260
avg_branching = 11.792142
propagation_edges = 4567032875
propagation_ns_per_edge = 92.698
solve_wall_ns_per_legal_move = 1208.231
push_wall_secs = 12385.279855
bfs_propagation_secs = 423.353546
audit_bad_positions = 0
push_vs_pull_mismatches = skipped
validation_label = PASS
```

## Timing / Resource Use

From `/usr/bin/time -v`:

```text
Elapsed wall time = 4:21:08
User time = 14520.61 s
System time = 240.48 s
CPU = 94%
Maximum resident set size = 63486272 KB (~60.5 GiB)
Major page faults = 4199621
Minor page faults = 51594308
Voluntary context switches = 4300289
Involuntary context switches = 77253
File system inputs = 111015136
File system outputs = 33956584
Exit status = 0
```

During manual progress checks, swap usage peaked at at least 32 GiB. `/usr/bin/time -v`
reported `Swaps: 0`; that field is not the same as peak swap-file occupancy.
A post-run kernel journal check for OOM-killer messages since `2026-06-11 00:00 UTC`
returned no matches.

Post-run table histogram scan:

```text
Command = solver/target/release/table_stats research/runs/kpg-2026-06-11/kpg.table.bin
Elapsed wall time = 0:25.87
Maximum resident set size = 55396 KB
Output = research/runs/kpg-2026-06-11/table-stats.txt
```

## Growth Over KP

KP baseline:

```text
positions = 457993
total_legal_moves = 3143376
avg_branching = 6.863371
```

KPG growth:

```text
positions ratio = 1898.04x
total legal moves ratio = 3261.07x
avg branching ratio = 1.72x
```

## Table Artifact

The raw binary table was intentionally not copied back locally. A compressed
backup was copied back and is ignored by git.

Remote path:

```text
/root/micro-shogi/research/runs/kpg-2026-06-11/kpg.table.bin
```

Local compressed backup:

```text
research/runs/kpg-2026-06-11/kpg.table.bin.zst
```

Raw metadata:

```text
bytes = 17385741380
records = 869287068
record_bytes = 20
format = MSHTB001: u64 record_count, u32 record_bytes, then repeated u128_le key + i32_le value
header bytes = 4d 53 48 54 42 30 30 31 9c 44 d0 33 00 00 00 00 14 00 00 00
sha256 = 470aafc38427790e7b8fe65187c3eee64849d0b1a78f7bb064d2e2ff9ca8c50f
```

Compressed backup metadata:

```text
command = zstd -T0 -6 -f -o kpg.table.bin.zst kpg.table.bin
compressed_bytes = 2181879983
compressed_size = 12.55% of raw table
remote_compression_wall_time = 0:20.74
remote_compression_max_rss = 129496 KB
rsync_transfer_time = about 3:26
zstd_test = PASS on remote and local
sha256 = 2adf6533272b1430b4e6dd224009ab01af66a94361000aa92a18657e3fd0be1f
```

Small artifact checksums:

```text
b3f5952cdb5e2ddd0441017581b7e308cb34f3bb8576f02546987c890db49328  kpg.json
ec539e5c6916b41ee68907556bfba43aeb5e3478886f59be02bfa58d60631975  kpg.txt
b5a6aec0f6f077520e418371ad2ef0c37c04c4fd8a203320dd4e49894d64ec14  console.log
0d7e9458105da5742c43da26efd559c2079b9f19a1b47952a620c0d144656dd3  console.overflow.log
6265389ce3e27b06d1725ccf0ccba4c8709bbc72da4427c56f4977421b10401a  table-stats.txt
```

## Interpretation

- KPG is much larger than the earlier ~130M estimate: 869M canonical reachable positions.
- The DTM tail is real but thin: median decided DTM is 2, p99 is 54, p99.9 is 89,
  and only 11 winning positions reach DTM 155.
- The current reachable-HashMap + stored-CSR solver can complete KPG on 64 GiB, but only
  barely. It is not a comfortable production method.
- The failed first attempt exposed a `u32` predecessor-offset overflow. The corrected run
  confirms the stored predecessor graph is the memory driver.
- KPGS should not be attempted with this driver. The next serious solver should inherit the
  Shogi4 dense-rank + on-demand predecessor design.
