# Dense-rank KPG run summary - 2026-06-12

Reduced-piece Micro Shogi rung: K + P + G per side.

This run compares the first KPG-specific dense-rank + on-demand predecessor
solver against the earlier reachable-HashMap + stored-CSR KPG solver. It is
superseded for headline numbers by the mirror-aware rerun in
`../dense-kpg-mirror-2026-06-12/`, but remains useful because it exposed the
2x mirror-duplication issue.

Command:

```sh
solver/target/release/solve_dense_kpg --out-dir research/runs/dense-kpg-2026-06-12
```

Remote host:

- Provider / model: Hetzner vServer, KVM
- Hostname: `ubuntu-64gb-fsn1-1`
- OS: Ubuntu 26.04 LTS
- Kernel: Linux 7.0.0-15-generic
- CPU: 16 vCPU, AMD EPYC-Milan Processor, 8 cores / 16 threads, 32 MiB L3
- RAM: 61 GiB
- Swap: 64 GiB `/swapfile`

Important source note: `/root/micro-shogi` on the remote was a source copy, not a
git checkout. The successful run used the local `dense-rank-kpg` branch copied to
the remote before this branch was committed.

## Result

```text
method = dense-rank-kpg
raw_rank_domain = 2037557340
positions_reachable_canonical = 869287068
rank_domain / reachable = 2.34394
start_id = 1647507943
start_value = 0
verdict = draw
wins / losses / draws = 606922331 / 142074547 / 120290190
max_dtm = 155
total_legal_moves = 10250756260
avg_branching = 11.792142
propagation_predecessor_updates = 3053600053
predecessor_candidates = 16317988874
predecessor_ids_after_dedup = 8158994437
duplicate_predecessor_ids = 8158994437
audit = skipped
```

The values match the audited CSR KPG run exactly:

```text
positions, start value, W/L/D, max DTM, total legal moves, avg branching = MATCH
```

## Timing / Resource Use

From dense solver instrumentation:

```text
enumeration_secs = 3150.526274
classification_secs = 268.656196
propagation_secs = 9229.233733
stats_secs = 60.072450
total_secs = 12708.934428
```

From `/usr/bin/time -v`:

```text
Elapsed wall time = 3:31:48
User time = 12700.39 s
System time = 7.42 s
CPU = 99%
Maximum resident set size = 7198160 KB (~6.86 GiB)
Major page faults = 0
Minor page faults = 2342897
Voluntary context switches = 4
Involuntary context switches = 86826
Swaps = 0
File system inputs = 16
File system outputs = 8
Exit status = 0
```

A post-run kernel journal check for OOM-killer messages since
`2026-06-12 06:30 UTC` returned no matches.

## Comparison With Stored-CSR KPG

Stored-CSR KPG baseline:

```text
core_solve_secs = 12385.279855
end_to_end_wall = 4:21:08 (15668 s), including table dump and --audit-large
max_rss = 63486272 KB (~60.55 GiB)
```

Dense-rank KPG comparison:

```text
total_secs = 12708.934428
end_to_end_wall = 3:31:48
max_rss = 7198160 KB (~6.86 GiB)
dense_vs_csr_full_run_speedup = 1.23283x
dense_vs_csr_core_solve_time = 1.02613x
rss_reduction = 8.81979x
```

Interpretation:

- Dense rank + on-demand predecessor generation reproduces the audited KPG
  result with no reachable-key `HashMap` and no stored predecessor CSR.
- The memory reduction is the main result: ~6.86 GiB instead of ~60.55 GiB.
- Single-thread core time is roughly comparable, not dramatically faster:
  dense was about 2.6% slower than CSR's core solve, but faster than the CSR
  run that included table dump and audit.
- The current dense predecessor generator does extra mirror work:
  `predecessor_ids_after_dedup == duplicate_predecessor_ids`, so about half of
  generated predecessor ids are discarded as duplicates. This is an obvious next
  optimization before treating the dense runtime as final.

## Checksums

```text
1aacb9238e89eca9695264f445600d55db43c28e9517849106da6be3ecefa833  console.log
e8f1bf341d785109555efd5a39eb2655de301627c1e2ccaeb574931470a7bb76  dense-kpg.json
```
