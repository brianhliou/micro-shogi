# Funding & collaboration — strategy and live programs (2026)

From a verified deep-research pass (2026-06-05): 24 sources, 3-vote adversarial
verification, 23/25 claims confirmed. **Program terms change frequently —
re-verify at application time.** Optimizing for two goals the researcher chose:
**(1) credential/validation** and **(2) scale insurance** against cost overruns.

## The core finding: funding splits on the affiliation line

The two goals are **in tension for a solo, non-affiliated person.** Nearly
everything that delivers real *scale insurance* (national HPC, academic cloud
credits) is hard-gated on institutional affiliation and explicitly rejects
personal/gmail email. The paths open to a solo individual deliver *credential*
but not capped-cost insurance. **The move that resolves both at once is an
academic co-author** — it is the keystone, not a nice-to-have.

## Solo-accessible (no affiliation required)

- **Emergent Ventures** (Mercatus / Tyler Cowen) — no affiliation, age 13+,
  worldwide; funds wired to a personal account; <1 week paperwork; one-pager
  annual report. Real credential value. **But: no published award ceiling**
  (the form states verbatim "There is NO typical grant range or average award
  amount – funding is project specific") → credential + seed cash, *not* scale
  insurance. Fit caveat: EV leans "scalable / improving society"; this project is
  "beautiful but not obviously impactful," so the pitch must lean on the
  *researcher's trajectory* + methods novelty, not the result's utility.
- **Cloudflare R2** — zero egress (one 18 TB download = $0 vs ~$1,620 on S3 at
  ~$0.09/GB). The artifact-distribution answer. (Whether their OSS *sponsorship*
  admits an individually-maintained project is unresolved — the restriction claim
  was refuted, so it's genuinely open.)

## Affiliation-gated (same gate every time)

| Program | Status | Why closed to a solo individual |
|---|---|---|
| NSF ACCESS (ex-XSEDE) | active | Unaffiliated barred from leading *or* collaborating; gmail prohibited; **co-PI circumvention explicitly forbidden** (tightened Sept 2025). **Maximize tier is uncapped** — the largest scale-insurance ceiling found anywhere, *if* affiliation is solved. Windows: Dec 15–Jan 31, Jun 15–Jul 31. |
| NSF Horizon (TACC LCCF) | active | Double-gated: PI-only *and* must show "current peer-reviewed research funding." Window opens ~Apr 15, 2026. |
| NAIRR Pilot | active | Institutional email required; no gmail. |
| Google Cloud Research Credits | active | Affiliation required *and* undersized ($5k faculty / $1k PhD). |
| AWS Cloud Credit for Research | **active** (not discontinued) | Requires .edu/.org email; "non-institution domains will not be accepted." |
| Oracle for Research | **retired** | Project Awards folded into general sales. Not viable. |
| NSF SBIR | active | Requires forming a registered business + a commercialization thesis the project lacks. Low fit. |

Every gated path collapses to one prerequisite: **acquire an affiliation, or
recruit a faculty co-PI who is the actual applicant.**

## The keystone: academic co-author (the load-bearing action)

Delivers *both* goals — credential (co-publication) **and** scale insurance
(their affiliation unlocks ACCESS Maximize / Horizon-class HPC, uncapped, free).

- **Robert Clausecker (Zuse Institute Berlin)** — author of `clausecker/dobutsu`,
  the *exact* tablebase lineage Micro Shogi extends. Best topical match. Caveat:
  ZIB status listed as "Guest" (scholarship-based), so his ability to grant
  NHR@ZIB / HLRN HPC access is unconfirmed — verify before relying on it.
- **UAlberta GAMES Group** — Jonathan Schaeffer (solved Checkers 2007 via
  retrograde-analysis endgame databases — *this project's exact technique*),
  Nathan Sturtevant (current prof; large-scale parallel heuristic/combinatorial
  search), Akihiro Kishimoto (shogi). Strongest North American institutional
  anchor.

**The unlock for outreach: the partial EGTB is the warm-intro artifact.** Not
"want to collaborate?" but "I've validated the state space (3.9×10¹⁵), built a
partial tablebase, here's the distributed external-memory architecture for the
full solve — want to co-author it on your cluster?" To someone whose research
*is* this, that's close to irresistible.

## Credential venues

- **Computers and Games (CG) 2026** — Maastricht, **June 19–20, 2026**, Springer
  LNCS. CFP welcomes "new and enhanced algorithms for search" and "mathematical
  insights into games." Track record: Pentago (strongly solved), Nine Men's
  Morris, 7×7 Killall-Go, Breakthrough tablebases. The prime fit.
- **ICGA Journal** — field journal for solved games / endgame tablebases (Awari,
  etc.). The archival credential.
- **Advances in Computer Games (ACG)** — alternating ICGA series.

## Recommended stack + sequence

1. **Build the partial EGTB solo** (~$0) — the artifact that makes every ask
   credible and converts the cost estimate into a measured number.
2. **Publish** the validated numbers + partial result + architecture.
3. **Recruit the co-author** (Clausecker for topical fit; UAlberta for
   institutional muscle) — the keystone: credential + uncapped HPC.
4. **Emergent Ventures in parallel** — no-strings seed + independent credential.
5. **Cloudflare R2** for artifact distribution.

The funding path is bottlenecked by the partial EGTB, not by which form gets
filled out. Build that, and the rest becomes gettable instead of speculative.

## Open questions (unresolved by the research pass)

- Microsoft Azure research/AI credits — current individual eligibility + max award.
- Cloudflare Project Alexandria / OSS sponsorship — does it admit an
  individually-maintained OSS project (OSI license) for in-kind R2 beyond the
  standard zero-egress pricing?
- Can Clausecker (Guest status) actually grant NHR@ZIB / HLRN access? EU HPC paths
  (EuroHPC LUMI / Leonardo) uninvestigated.
- Lightest-weight affiliation (visiting/adjunct/affiliate scholar) that satisfies
  ACCESS/Horizon's "eligible organization" + institutional-email requirement —
  faster than recruiting a co-PI?
- Other independent-researcher funders not in the verified set: Astera Residency,
  Experiment.com (crowdfunding), Manifund / EA-adjacent regrantors.

## Key sources

- EV: <https://www.mercatus.org/emergent-ventures> · <https://mercatus.tfaforms.net/5099527>
- ACCESS: <https://allocations.access-ci.org/allocations-policy> · <https://allocations.access-ci.org/project-types>
- Horizon/LCCF: <https://lccf.tacc.utexas.edu/allocations/> · NAIRR: <https://nairrpilot.org/opportunities/allocations>
- Google: <https://edu.google.com/intl/ALL_us/programs/credits/research/> · AWS: <https://aws.amazon.com/government-education/research-and-technical-computing/cloud-credit-for-research/>
- Cloudflare R2: <https://developers.cloudflare.com/r2/pricing>
- Clausecker: <https://github.com/clausecker/dobutsu> · <https://www.zib.de/members/clausecker>
- UAlberta GAMES: <https://www.ualberta.ca/en/computing-science/research/research-areas/computer-games.html> · Schaeffer: <http://webdocs.cs.ualberta.ca/~jonathan/>
- CG 2026: <https://icga.org/?page_id=4105> · <https://easychair.org/cfp/CG_2026> · ICGA Journal: <https://journals.sagepub.com/home/icga>
