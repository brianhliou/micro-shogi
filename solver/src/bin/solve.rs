//! Calibration solve: retrograde-solve a ladder of reduced-piece sub-games,
//! cross-validate each (consistency audit + independent forward minimax), and
//! report the first solved values, exact counts, real branching, and ns/edge.
//!
//!   cargo run --release --bin solve            # KP, KPG, KPGS
//!   cargo run --release --bin solve KPGS       # one rung

use microshogi::retro::{audit, rung_start, solve};
use microshogi::{format, unpack};
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
        let s = solve(&start);
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
        let total_moves: u64 = (0..n).map(|i| unpack(s.keys[i]).moves().len() as u64).sum();
        let bf = total_moves as f64 / n as f64;
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
        println!("  fixpoint rounds                   : {}", s.rounds);
        println!("  edge-ops / ns-per-edge            : {} / {ns_edge:.0} ns", s.edges);
        println!("  solve wall-clock                  : {total:?}");
        let bad = audit(&s);
        println!(
            "  VALIDATION  consistency-audit={}  -> {}",
            bad,
            if bad == 0 { "PASS" } else { "FAIL" }
        );
    }
}
