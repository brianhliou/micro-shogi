# Mirror-aware dense-rank KPG run summary - 2026-06-12/13

Reduced-piece Micro Shogi rung: K + P + G per side.

This run measures the KPG-specific dense-rank + on-demand predecessor solver
after removing the redundant mirror-child predecessor pass.

Command:

```sh
solver/target/release/solve_dense_kpg --out-dir research/runs/dense-kpg-mirror-2026-06-12
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
predecessor_candidates = 8158994437
predecessor_ids = 8158994437
duplicate_predecessor_ids = 0
audit = skipped
```

The values match the audited CSR KPG run exactly:

```text
positions, start value, W/L/D, max DTM, total legal moves, avg branching = MATCH
```

## Timing / Resource Use

From dense solver instrumentation:

```text
enumeration_secs = 3183.993376
classification_secs = 259.037162
propagation_secs = 4960.590507
stats_secs = 59.982444
total_secs = 8464.046029
```

From `/usr/bin/time -v`:

```text
Elapsed wall time = 2:21:04
User time = 8456.55 s
System time = 6.80 s
CPU = 99%
Maximum resident set size = 7198128 KB (~6.86 GiB)
Major page faults = 0
Minor page faults = 2342891
Voluntary context switches = 1
Involuntary context switches = 52069
Swaps = 0
File system inputs = 0
File system outputs = 8
Exit status = 0
```

A post-run kernel journal check for OOM-killer messages since
`2026-06-12 22:20 UTC` returned no matches.

## Comparison With Stored-CSR KPG

Stored-CSR KPG baseline:

```text
core_solve_secs = 12385.279855
end_to_end_wall = 4:21:08 (15668 s), including table dump and --audit-large
max_rss = 63486272 KB (~60.55 GiB)
```

Mirror-aware dense-rank KPG:

```text
total_secs = 8464.046029
end_to_end_wall = 2:21:04
max_rss = 7198128 KB (~6.86 GiB)
dense_vs_csr_full_run_speedup = 1.85112x
dense_vs_csr_core_solve_speedup = 1.46328x
rss_reduction = 8.81983x
```

The first dense run generated both mirror orientations, then canonical-deduped
the predecessor ids. This mirror-aware run removed that waste:

```text
first_dense_total_secs = 12708.934428
mirror_aware_total_secs = 8464.046029
total_speedup = 1.50152x
first_dense_propagation_secs = 9229.233733
mirror_aware_propagation_secs = 4960.590507
propagation_speedup = 1.86051x
duplicate_predecessor_ids = 8158994437 -> 0
```

Interpretation:

- Dense rank + on-demand predecessor generation reproduces the audited KPG
  result with no reachable-key `HashMap` and no stored predecessor CSR.
- Memory falls from ~60.55 GiB to ~6.86 GiB.
- After mirror-aware predecessor generation, the dense solver is also faster on
  this single-core KPG comparison: 2:21:04 versus the CSR core solve's 3:26:25.
- The dense solver still spends more CPU per predecessor than walking a stored
  CSR, but it avoids the expensive reverse-graph construction and the memory
  pressure that made CSR marginal on a 64 GB box.

## Checksums

```text
d06a940325201de92d90d7b998788d90e6f1df420026dc5335845f8f25e0aa4d  console.log
3aa481d34681a5f894811e65143666154f42fcd40dee4b162560b1809a3aa58a  dense-kpg.json
```

