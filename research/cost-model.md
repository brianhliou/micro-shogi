# Cost model — solving the complete Micro Shogi tablebase

All figures are **estimates** built on the validated state-space count (~5×10¹⁴ canonical
positions) and 2026 cloud/bare-metal pricing (sources at bottom; spot prices are point-in-time
and volatile). The compute estimate carries ~10× uncertainty until the partial-EGTB milestone
calibrates it.

> ⚠ **Sizing basis correction (see `findings.md` → "the index domain is larger").** Everything
> below is sized to the **reachable** count (~5×10¹⁴). A combinatorial-rank solver (the only
> scalable index; MPHF-over-reachable is infeasible at this scale) stores a slot per **arrangement**,
> ~7.8× more (~3.9×10¹⁵ LR-folded). On that basis the working set is **~1 PB**, compute
> **~660–1,200 core-years**, cost **~$40–70k bare-metal (~$150–280k cloud)** — multiply the figures
> below by ~7–8×. The reachable-based numbers below remain the *compressed-artifact* and
> *floor* values. Carried from `../shogi4`, 2026-06-05.

## What the job needs

| Resource | Quantity | Notes |
|---|---|---|
| Compute | **~150 core-years** (range 40–475) | 5×10¹⁴ pos × ~16 edges × passes × ~300 ns/edge |
| Working storage | **134 TB** (W/L/D) – 668 TB (DTM) | on cluster NVMe |
| Cumulative shuffle | **~100 PB** | a *throughput* requirement, not a billed line — if kept off metered links |
| Wall time | ~1–2 months | binding constraint is CPU |
| Nodes | ~16–20 NVMe-dense | CPU-sized; I/O + storage fit with headroom |

## The key insight: what you actually pay for

The ~100 PB shuffle is **not a cost line** if you keep it **node-local** (bare-metal) or
**same-AZ over private IPs** (AWS = $0). The budget therefore reduces to:

1. **Compute-hours** (dominant) = core-years × $/core-hr.
2. **Durable storage-months** (secondary) = TB × $/TB-mo × duration.
3. **Egress** = the trap. Engineer it to ~$0; if you don't, it dwarfs everything.

## Compute cost — the decomposition

The whole compute bill is one product:

    cost = (edge-operations) × (time per edge-op) × ($/core-second)

Two factors are pinned; one carries the entire ~10× uncertainty.

**Edge-operations — pinned at ~1.6×10¹⁶.** Retrograde must touch every
(position, move) edge to know the value — the information-theoretic floor.
positions (~5–6×10¹⁴; all-legal ≈ 1.2× reachable) × avg-branching (~16; Dōbutsu
*measured* 9.435) × ~2 passes (push-based: one forward to set each position's
undecided-children counter, one backward to propagate). Solid to ±2×. Caveat: a
*pull-based* Jacobi solve (rescan all unknowns every round — our Dōbutsu
approach) balloons this ~5–10×, so push-vs-pull is a free 5–10× lever.

**$/core-hour — a known menu.** Hetzner bare-metal ~$0.006; GCP spot ~$0.015;
AWS storage-opt spot ~$0.0275.

**Time per edge-op — the entire spread, 150 ns → 100,000 ns.** An edge-op is
decode → (un)move-gen → canonicalize the neighbor key → **look up its value** →
decrement/compare. The lookup dominates: RAM-resident ~150 ns (*measured* — the
Dōbutsu solver's optimized rate); random SSD ~10,000–100,000 ns, a 100–1000×
cliff. At 5×10¹⁴ the value array does NOT fit in RAM, so the effective rate is
set entirely by whether the external-memory architecture keeps access
sequential/RAM-speed or lets it degrade to disk-random. **This is the dominant
risk — see `architecture.md` → "The central risk".**

### Cost matrix (bare-metal $0.006/core-hr; ×4.5 for AWS spot)

| Scenario | ns/edge | core-years | bare-metal $ | AWS spot $ |
|---|---|---|---|---|
| **Floor** — push, sequential streaming, RAM-speed | 150 | ~76 | **~$4k** | ~$18k |
| **Realistic** — push + shuffle overhead | 300–500 | ~150–250 | **~$8–13k** | ~$36–58k |
| **Pessimistic** — pull-based *or* partial random I/O | ~1,000 | ~475 | **~$25k** | ~$115k |
| Disaster — naive random disk | 10,000+ | thousands | infeasible | infeasible |

Linear in core-years: compute is buyable — more nodes = proportionally less
wall-clock at the same dollar total. You're renting a fixed amount of work, not
fighting a wall.

### The calculator

    core-years = positions × branching × passes × ns_per_edge / 3.15576e16
    dollars    = core-years × 8766 × ($/core-hr)

Sanity check: 5e14 × 16 × 2 × 150 / 3.15576e16 ≈ 76 core-years → ×8766×0.006 ≈
$4.0k. Dial the four assumptions; everything rides on **ns_per_edge**, which the
partial-EGTB milestone measures directly for ~$40 — collapsing the 76–475
core-year range to a single number ±20%. The highest-leverage spend in the
project.

**W/L/D-first cuts compute too**, not just storage: a 2-bit verdict pass is
cheaper per edge than carrying/updating DTM, and you skip the DTM-fill pass
entirely. One lever, both axes.

## Pricing inputs (2026)

**Compute, $/vCPU-hr:**

| Provider | Mode | $/vCPU-hr |
|---|---|---|
| Hetzner AX162 (dedicated) | bare-metal flat | **~$0.006** (48c/96t @ ~$215/mo) |
| OVH Advance (EPYC) | bare-metal flat | ~$0.005–0.01 |
| GCP | spot/preemptible | ~$0.014–0.017 |
| AWS storage-opt | spot | ~$0.026–0.045 |

**AWS storage-optimized instances (local NVMe + bandwidth):**

| Instance | vCPU | RAM | NVMe | Net | spot $/hr |
|---|---|---|---|---|---|
| im4gn.16xlarge | 64 | 256 GiB | 30 TB | 100 Gbps | ~$1.76 (cheapest/vCPU) |
| i3en.24xlarge | 96 | 768 GiB | 60 TB | 100 Gbps | ~$3.86 (most NVMe/$) |
| i4i.32xlarge | 128 | 1024 GiB | 30 TB | 75 Gbps | ~$4.31 |

**Object storage $/TB-mo:** Backblaze B2 **$6** · Cloudflare R2 $15 · S3-IA ~$12.5 · S3 $23.

**Egress:** AWS→internet $0.07–0.09/GB · AWS inter-AZ $0.02/GB round-trip ·
**AWS same-AZ private IP = $0** · **R2 egress = $0** · B2 egress free via Cloudflare alliance.

**Network:** AWS i3en/im4gn up to 100 Gbps intra-AZ · Hetzner 1 Gbps included, 10 Gbps add-on.

## The two deployments, costed (W/L/D target, 134 TB)

| | **Hetzner bare-metal** (recommended) | **AWS storage-opt spot, single-AZ** |
|---|---|---|
| nodes | ~16× AX162-S (48c/96t, 30 TB NVMe ea.) | ~15× im4gn.16xlarge (64 vCPU, 30 TB, 100 Gbps) |
| $/core-hr | ~$0.006 (thread) | ~$0.0275 |
| wall time | ~2 months | ~6 weeks |
| compute | ~$6.9k + ~$2.8k setup | ~$27k (i3en variant ~$50k) |
| network | 10 Gbps add-on ~$1k | 100 Gbps incl., same-AZ free |
| durable store | on-box ~$0 (or B2 ~$1–2k) | B2 checkpoints ~$2–3k |
| **egress** | **$0** | **$9.4k** to pull 134 TB out (or $3.1k/mo to keep in S3) |
| **total (W/L/D)** | **~$10–12k** | **~$40–50k** |
| total (full DTM, 668 TB) | ~$18–25k | ~$60–90k+ |

**Bare-metal wins ~4×**, structurally: no spot interruption over a multi-week run, NVMe
flat-rate, **zero egress**. The only thing that flips it to AWS: if the shuffle genuinely needs
>10 Gbps bisection bandwidth and can't be localized — Hetzner caps at 10 Gbps/node, AWS does
100 Gbps same-AZ for free. **So the architecture decision (can shuffle stay node-local?) maps
directly to the $.**

## Cost levers (each large)

1. **W/L/D-only persisted artifact** (no stored DTM): 134 TB not 668 TB; optimal play still
   recoverable via probe-time local search. Compresses to a ~20–40 TB downloadable. **~8×.**
2. **Stream-and-discard via SCC staging:** evict solved buckets after they propagate; peak
   resident storage ~10–30 TB instead of 134 TB–1 PB. Size purely to CPU. **~10× on peak storage.**
3. **Bare-metal over cloud:** ~4–5× on the total, driven by spot premium + egress.
4. **Stronger symmetry folding:** up to ~4× fewer positions (our solver currently leaves most
   of this on the table); even 1.5–2× is meaningful at this scale.

## Recommendation

**Hetzner bare-metal, W/L/D-only, stream-and-discard, node-local shuffle. Budget ~$10–15k,
~2 months, ~16 boxes** for the full run — **but do not commit blind.** The partial-EGTB
milestone runs on one ~$40 Hetzner auction box (or local hardware), validates correctness, and
calibrates the real per-edge cost, collapsing the 40–475 core-year range. **The entire pipeline
can be de-risked for ~$0–50 before the $10k+ decision.**

## Sources

- AWS [EC2 Spot](https://aws.amazon.com/ec2/spot/pricing/) ·
  [i3en](https://aws.amazon.com/ec2/instance-types/i3en/) ·
  [i4i](https://aws.amazon.com/ec2/instance-types/i4i/) ·
  [data-transfer 2026](https://leanopstech.com/blog/aws-data-transfer-pricing-2026/)
- [GCP Spot VMs](https://cloud.google.com/spot-vms/pricing)
- Hetzner [AX162-R](https://www.hetzner.com/dedicated-rootserver/ax162-r/) ·
  [AX matrix](https://www.hetzner.com/dedicated-rootserver/matrix-ax/) ·
  [auction](https://www.hetzner.com/sb/) ·
  [June 2026 price adjustment](https://www.hetzner.com/pressroom/standardization-and-price-adjustment-of-our-server-products/)
- [OVH bare-metal](https://www.ovhcloud.com/en/bare-metal/prices/)
- [Cloudflare R2](https://developers.cloudflare.com/r2/pricing/) ·
  [Backblaze B2](https://www.backblaze.com/cloud-storage/pricing)
