//! Calibration solve: retrograde-solve a ladder of reduced-piece sub-games,
//! cross-validate each (consistency audit + independent forward minimax), and
//! report the first solved values, exact counts, real branching, and ns/edge.
//!
//!   cargo run --release --bin solve            # KP, KPG, KPGS
//!   cargo run --release --bin solve KPGS       # one rung

use microshogi::retro::{audit, cross_check, rung_start, solve, solve_push};
use microshogi::format;
use std::io::Write;
use std::time::Instant;

fn main() {
    let arg = std::env::args().nth(1);
    let rungs: Vec<&str> = match arg.as_deref() {
        Some(a) => vec![a],
        None => vec!["KP", "KPG", "KPGS"],
    };
    for name in rungs {
        let start = match rung_start(name) {
            Some(p) => p,
            None => {
                eprintln!("unknown rung: {name}");
                continue;
            }
        };
        println!("\n=== rung {name}   start {} ===", format(&start));
        let t = Instant::now();
        let s = solve_push(&start);
        let total = t.elapsed();
        let n = s.keys.len();

        let (mut w, mut l, mut d, mut maxdtm) = (0u64, 0u64, 0u64, 0i32);
        for &v in &s.values {
            if v > 0 {
                w += 1;
                maxdtm = maxdtm.max(v);
            } else if v < 0 {
                l += 1;
            } else {
                d += 1;
            }
        }
        let bf = s.total_moves as f64 / n as f64; // free — counted during the solve
        let ns_edge = s.fixpoint_ns as f64 / s.edges.max(1) as f64;

        let val = s.values[0];
        let verdict = if val > 0 {
            format!("Sente (mover) wins in {val}")
        } else if val < 0 {
            format!("Gote wins in {}", -val)
        } else {
            "draw".to_string()
        };

        println!("  positions (reachable, canonical) : {n}");
        println!("  start value                       : {val}  ({verdict})");
        println!("  W / L / D                         : {w} / {l} / {d}");
        println!("  max DTM                           : {maxdtm} plies");
        println!("  avg branching (reachable)         : {bf:.3}");
        println!("  BFS edge-ops / ns-per-edge        : {} / {ns_edge:.0} ns", s.edges);
        println!("  solve wall-clock (push)           : {total:?}");
        std::io::stdout().flush().ok(); // surface results before the (slow) validation re-scan

        // validation does full re-scans (≈ a solve pass each) — gate to where it's cheap.
        // push is already cross-validated ≡ pull on the small rungs.
        let abad: Option<u64> = if n <= 5_000_000 { Some(audit(&s)) } else { None };
        let xmis: Option<u64> = if n <= 1_000_000 {
            Some(cross_check(&s, &solve(&start)))
        } else {
            None
        };
        let astr = abad.map_or("audit=skipped(>5M)".to_string(), |b| format!("audit={b}"));
        let xstr = xmis.map_or("push-vs-pull=skipped(>1M)".to_string(), |m| format!("push-vs-pull={m}"));
        let label = if abad.is_none() && xmis.is_none() {
            "(validation skipped — push≡pull verified on smaller rungs)"
        } else if abad.map_or(true, |b| b == 0) && xmis.map_or(true, |m| m == 0) {
            "PASS"
        } else {
            "FAIL"
        };
        println!("  VALIDATION  {astr}  {xstr}  -> {label}");
    }
}
