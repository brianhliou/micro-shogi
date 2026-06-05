//! perft — count legal move sequences from the start to a given depth, and report
//! the implied average branching factor. A self-consistency check on move
//! generation, and an empirical measurement of branching to compare against the
//! cost model's ~16 estimate (research/cost-model.md).
//!
//!   cargo run --release --bin perft [max_depth]   (default 6)

use microshogi::{initial, perft};
use std::time::Instant;

fn main() {
    let max: u32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(6);
    let p = initial();
    println!("start: {}", microshogi::format(&p));
    let mut prev = 1u64;
    for d in 1..=max {
        let t = Instant::now();
        let n = perft(&p, d);
        let bf = n as f64 / prev as f64;
        println!(
            "perft({d}) = {n:>16}   bf≈{bf:6.2}   ({:?})",
            t.elapsed()
        );
        prev = n;
    }
}
