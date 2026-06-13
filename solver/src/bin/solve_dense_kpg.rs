//! Dense-rank KPG comparison solve.

use microshogi::dense_kpg::{domain_size, solve_dense_kpg};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

fn main() {
    let config = parse_args();
    let t = Instant::now();
    let solved = solve_dense_kpg(config.audit_large);
    let total_secs = t.elapsed().as_secs_f64();

    println!("=== dense-rank KPG ===");
    println!(
        "  raw rank domain                   : {}",
        solved.domain_size
    );
    println!("  positions (reachable, canonical)  : {}", solved.positions);
    println!(
        "  start id / value                  : {} / {}",
        solved.start_id, solved.start_value
    );
    println!(
        "  W / L / D                         : {} / {} / {}",
        solved.wins, solved.losses, solved.draws
    );
    println!(
        "  max DTM                           : {} plies",
        solved.max_dtm
    );
    println!(
        "  total legal moves                 : {}",
        solved.total_moves
    );
    println!(
        "  avg branching (reachable)         : {:.6}",
        solved.total_moves as f64 / solved.positions as f64
    );
    println!(
        "  propagation predecessor updates   : {}",
        solved.propagation_edges
    );
    println!(
        "  predecessor candidates / ids      : {} / {}",
        solved.predecessor_candidates, solved.predecessor_ids
    );
    println!(
        "  duplicate predecessor ids         : {}",
        solved.duplicate_predecessor_ids
    );
    println!(
        "  enumerate / classify / propagate  : {:.3}s / {:.3}s / {:.3}s",
        solved.enumeration_secs, solved.classification_secs, solved.propagation_secs
    );
    println!(
        "  stats scan                        : {:.3}s",
        solved.stats_secs
    );
    if let Some(bad) = solved.audit_bad_positions {
        println!(
            "  audit                             : {bad} bad positions ({:.3}s)",
            solved.audit_secs.unwrap_or_default()
        );
    } else {
        println!("  audit                             : skipped");
    }
    println!("  total wall-clock                  : {total_secs:.3}s");

    if let Some(out_dir) = config.out_dir {
        if let Err(err) = fs::create_dir_all(&out_dir) {
            eprintln!("failed to create {}: {err}", out_dir.display());
            process::exit(1);
        }
        let report = json_report(&solved, total_secs);
        let path = out_dir.join("dense-kpg.json");
        if let Err(err) = fs::write(&path, report) {
            eprintln!("failed to write {}: {err}", path.display());
            process::exit(1);
        }
    }
}

struct Config {
    out_dir: Option<PathBuf>,
    audit_large: bool,
}

fn parse_args() -> Config {
    let mut out_dir = None;
    let mut audit_large = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out-dir" => {
                let Some(path) = args.next() else {
                    eprintln!("--out-dir requires a path");
                    process::exit(2);
                };
                out_dir = Some(PathBuf::from(path));
            }
            "--audit-large" => audit_large = true,
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            _ => {
                eprintln!("unknown option: {arg}");
                print_usage();
                process::exit(2);
            }
        }
    }
    Config {
        out_dir,
        audit_large,
    }
}

fn print_usage() {
    eprintln!("usage: solve_dense_kpg [--out-dir PATH] [--audit-large]");
    eprintln!("raw rank domain: {}", domain_size());
}

fn json_report(solved: &microshogi::dense_kpg::DenseKpgSolved, total_secs: f64) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"method\": \"dense-rank-kpg\",\n",
            "  \"domain_size\": {domain_size},\n",
            "  \"positions_reachable_canonical\": {positions},\n",
            "  \"start_id\": {start_id},\n",
            "  \"start_value\": {start_value},\n",
            "  \"wins\": {wins},\n",
            "  \"losses\": {losses},\n",
            "  \"draws\": {draws},\n",
            "  \"max_dtm\": {max_dtm},\n",
            "  \"total_legal_moves\": {total_moves},\n",
            "  \"avg_branching\": {avg_branching:.6},\n",
            "  \"propagation_edges\": {propagation_edges},\n",
            "  \"predecessor_candidates\": {predecessor_candidates},\n",
            "  \"predecessor_ids\": {predecessor_ids},\n",
            "  \"duplicate_predecessor_ids\": {duplicate_predecessor_ids},\n",
            "  \"enumeration_secs\": {enumeration_secs:.6},\n",
            "  \"classification_secs\": {classification_secs:.6},\n",
            "  \"propagation_secs\": {propagation_secs:.6},\n",
            "  \"stats_secs\": {stats_secs:.6},\n",
            "  \"audit_bad_positions\": {audit_bad_positions},\n",
            "  \"audit_secs\": {audit_secs},\n",
            "  \"total_secs\": {total_secs:.6}\n",
            "}}\n",
        ),
        domain_size = solved.domain_size,
        positions = solved.positions,
        start_id = solved.start_id,
        start_value = solved.start_value,
        wins = solved.wins,
        losses = solved.losses,
        draws = solved.draws,
        max_dtm = solved.max_dtm,
        total_moves = solved.total_moves,
        avg_branching = solved.total_moves as f64 / solved.positions as f64,
        propagation_edges = solved.propagation_edges,
        predecessor_candidates = solved.predecessor_candidates,
        predecessor_ids = solved.predecessor_ids,
        duplicate_predecessor_ids = solved.duplicate_predecessor_ids,
        enumeration_secs = solved.enumeration_secs,
        classification_secs = solved.classification_secs,
        propagation_secs = solved.propagation_secs,
        stats_secs = solved.stats_secs,
        audit_bad_positions = solved
            .audit_bad_positions
            .map_or("null".to_string(), |v| v.to_string()),
        audit_secs = solved
            .audit_secs
            .map_or("null".to_string(), |v| format!("{v:.6}")),
        total_secs = total_secs,
    )
}
