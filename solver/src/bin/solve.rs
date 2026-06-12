//! Calibration solve: retrograde-solve a ladder of reduced-piece sub-games,
//! cross-validate each (consistency audit + independent forward minimax), and
//! report the first solved values, exact counts, real branching, and ns/edge.
//!
//!   cargo run --release --bin solve                         # KP
//!   cargo run --release --bin solve KPG --out-dir research/runs/kpg
//!   cargo run --release --bin solve KPG --out-dir research/runs/kpg --dump-table --audit-large
//!   cargo run --release --bin solve ALL                     # KP, KPG

use microshogi::format;
use microshogi::retro::{audit, cross_check, rung_start, solve, solve_push, Solved};
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

fn main() {
    let config = parse_args();
    let rungs: Vec<&str> = config.rungs.iter().map(String::as_str).collect();
    for name in rungs {
        let start = match rung_start(name) {
            Some(p) => p,
            None => {
                eprintln!("unknown rung: {name}");
                process::exit(2);
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
            } else if v < 0 {
                l += 1;
            } else {
                d += 1;
            }
            if v != 0 {
                maxdtm = maxdtm.max(v.abs());
            }
        }
        let bf = s.total_moves as f64 / n as f64; // free — counted during the solve
        let propagation_ns_edge = s.fixpoint_ns as f64 / s.edges.max(1) as f64;
        let wall_ns_move = total.as_nanos() as f64 / s.total_moves.max(1) as f64;

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
        println!("  total legal moves                 : {}", s.total_moves);
        println!(
            "  propagation edge-ops / ns-per-edge: {} / {propagation_ns_edge:.0} ns",
            s.edges
        );
        println!("  solve wall ns/legal-move          : {wall_ns_move:.0} ns");
        println!("  solve wall-clock (push)           : {total:?}");
        std::io::stdout().flush().ok(); // surface results before the (slow) validation re-scan
        let mut result = RunResult {
            rung: name,
            start: format(&start),
            positions: n,
            start_value: val,
            verdict,
            wins: w,
            losses: l,
            draws: d,
            max_dtm: maxdtm,
            total_moves: s.total_moves,
            avg_branching: bf,
            propagation_edges: s.edges,
            propagation_ns_per_edge: propagation_ns_edge,
            solve_wall_ns_per_legal_move: wall_ns_move,
            push_wall_secs: total.as_secs_f64(),
            fixpoint_secs: s.fixpoint_ns as f64 / 1_000_000_000.0,
            audit: None,
            cross_check: None,
            validation_label: "pending".to_string(),
            table_dump: None,
        };
        write_outputs(&config, &result);
        if config.dump_table {
            result.table_dump = dump_table(&config, &result, &s);
            write_outputs(&config, &result);
        }

        // validation does full re-scans (≈ a solve pass each) — gate to where it's cheap.
        // push is already cross-validated ≡ pull on the small rungs.
        let abad: Option<u64> = if n <= 5_000_000 || config.audit_large {
            Some(audit(&s))
        } else {
            None
        };
        let xmis: Option<u64> = if n <= 1_000_000 {
            Some(cross_check(&s, &solve(&start)))
        } else {
            None
        };
        let astr = abad.map_or("audit=skipped(>5M)".to_string(), |b| format!("audit={b}"));
        let xstr = xmis.map_or("push-vs-pull=skipped(>1M)".to_string(), |m| {
            format!("push-vs-pull={m}")
        });
        let label = if abad.is_none() && xmis.is_none() {
            "(validation skipped — push≡pull verified on smaller rungs)"
        } else if abad.map_or(true, |b| b == 0) && xmis.map_or(true, |m| m == 0) {
            "PASS"
        } else {
            "FAIL"
        };
        println!("  VALIDATION  {astr}  {xstr}  -> {label}");
        result.audit = abad;
        result.cross_check = xmis;
        result.validation_label = label.to_string();
        write_outputs(&config, &result);
    }
}

struct Config {
    rungs: Vec<String>,
    out_dir: Option<PathBuf>,
    audit_large: bool,
    dump_table: bool,
}

struct RunResult<'a> {
    rung: &'a str,
    start: String,
    positions: usize,
    start_value: i32,
    verdict: String,
    wins: u64,
    losses: u64,
    draws: u64,
    max_dtm: i32,
    total_moves: u64,
    avg_branching: f64,
    propagation_edges: u64,
    propagation_ns_per_edge: f64,
    solve_wall_ns_per_legal_move: f64,
    push_wall_secs: f64,
    fixpoint_secs: f64,
    audit: Option<u64>,
    cross_check: Option<u64>,
    validation_label: String,
    table_dump: Option<TableDump>,
}

struct TableDump {
    path: String,
    records: usize,
    bytes: u64,
    record_bytes: u32,
    format: &'static str,
}

fn parse_args() -> Config {
    let mut rung: Option<String> = None;
    let mut out_dir: Option<PathBuf> = None;
    let mut audit_large = false;
    let mut dump_table = false;
    let mut allow_huge = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            "--out-dir" => {
                let Some(path) = args.next() else {
                    eprintln!("--out-dir requires a path");
                    process::exit(2);
                };
                out_dir = Some(PathBuf::from(path));
            }
            "--audit-large" => audit_large = true,
            "--dump-table" => dump_table = true,
            "--allow-huge" => allow_huge = true,
            _ if arg.starts_with("--") => {
                eprintln!("unknown option: {arg}");
                print_usage();
                process::exit(2);
            }
            _ => {
                if rung.is_some() {
                    eprintln!("only one rung argument is supported");
                    print_usage();
                    process::exit(2);
                }
                rung = Some(arg);
            }
        }
    }
    if dump_table && out_dir.is_none() {
        eprintln!("--dump-table requires --out-dir PATH");
        process::exit(2);
    }

    let rungs = match rung.as_deref() {
        None => vec!["KP".to_string()],
        Some("ALL") => vec!["KP".to_string(), "KPG".to_string()],
        Some(other) => vec![other.to_string()],
    };
    for name in &rungs {
        let huge = matches!(name.as_str(), "KPGS" | "FULL" | "KPGSB");
        if huge && !allow_huge {
            eprintln!("{name} is out of scope for the in-RAM calibration driver; pass --allow-huge to override.");
            process::exit(2);
        }
    }

    Config {
        rungs,
        out_dir,
        audit_large,
        dump_table,
    }
}

fn print_usage() {
    eprintln!("usage: solve [KP|KPG|ALL] [--out-dir PATH] [--dump-table] [--audit-large]");
    eprintln!(
        "       solve KPGS --allow-huge   # intentionally guarded; expected to need TB-scale RAM"
    );
}

fn write_outputs(config: &Config, result: &RunResult<'_>) {
    let Some(dir) = &config.out_dir else {
        return;
    };
    if let Err(err) = fs::create_dir_all(dir) {
        eprintln!("failed to create output dir {}: {err}", dir.display());
        return;
    }
    write_file(
        &dir.join(format!("{}.txt", result.rung.to_ascii_lowercase())),
        &text_report(result),
    );
    write_file(
        &dir.join(format!("{}.json", result.rung.to_ascii_lowercase())),
        &json_report(result),
    );
}

fn write_file(path: &Path, content: &str) {
    let tmp = path.with_extension("tmp");
    if let Err(err) = fs::write(&tmp, content) {
        eprintln!("failed to write {}: {err}", tmp.display());
        return;
    }
    if let Err(err) = fs::rename(&tmp, path) {
        eprintln!(
            "failed to move {} to {}: {err}",
            tmp.display(),
            path.display()
        );
    }
}

fn dump_table(config: &Config, result: &RunResult<'_>, solved: &Solved) -> Option<TableDump> {
    let Some(dir) = &config.out_dir else {
        return None;
    };
    if let Err(err) = fs::create_dir_all(dir) {
        eprintln!("failed to create output dir {}: {err}", dir.display());
        return None;
    }

    let path = dir.join(format!("{}.table.bin", result.rung.to_ascii_lowercase()));
    match write_table(&path, solved) {
        Ok(bytes) => Some(TableDump {
            path: path.display().to_string(),
            records: solved.keys.len(),
            bytes,
            record_bytes: 20,
            format: "MSHTB001: u64 record_count, u32 record_bytes, then repeated u128_le key + i32_le value",
        }),
        Err(err) => {
            eprintln!("failed to dump table {}: {err}", path.display());
            None
        }
    }
}

fn write_table(path: &Path, solved: &Solved) -> std::io::Result<u64> {
    let tmp = path.with_extension("tmp");
    let file = File::create(&tmp)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(b"MSHTB001")?;
    writer.write_all(&(solved.keys.len() as u64).to_le_bytes())?;
    writer.write_all(&20u32.to_le_bytes())?;
    for (&key, &value) in solved.keys.iter().zip(&solved.values) {
        writer.write_all(&key.to_le_bytes())?;
        writer.write_all(&value.to_le_bytes())?;
    }
    writer.flush()?;
    drop(writer);
    fs::rename(&tmp, path)?;
    Ok(8 + 8 + 4 + (solved.keys.len() as u64 * 20))
}

fn text_report(result: &RunResult<'_>) -> String {
    let table = result
        .table_dump
        .as_ref()
        .map_or("not written".to_string(), |dump| {
            format!(
                "{} ({} records, {} bytes, {} bytes/record)",
                dump.path, dump.records, dump.bytes, dump.record_bytes
            )
        });
    format!(
        "\
=== rung {rung}   start {start} ===
positions (reachable, canonical) : {positions}
start value                       : {start_value}  ({verdict})
W / L / D                         : {wins} / {losses} / {draws}
max DTM                           : {max_dtm} plies
avg branching (reachable)         : {avg_branching:.3}
total legal moves                 : {total_moves}
propagation edge-ops / ns-per-edge: {propagation_edges} / {propagation_ns_per_edge:.0} ns
solve wall ns/legal-move          : {solve_wall_ns_per_legal_move:.0} ns
solve wall-clock (push)           : {push_wall_secs:.3}s
BFS propagation wall-clock        : {fixpoint_secs:.3}s
VALIDATION                        : audit={audit} push-vs-pull={cross_check} -> {validation_label}
table dump                        : {table}
",
        rung = result.rung,
        start = result.start,
        positions = result.positions,
        start_value = result.start_value,
        verdict = result.verdict,
        wins = result.wins,
        losses = result.losses,
        draws = result.draws,
        max_dtm = result.max_dtm,
        avg_branching = result.avg_branching,
        total_moves = result.total_moves,
        propagation_edges = result.propagation_edges,
        propagation_ns_per_edge = result.propagation_ns_per_edge,
        solve_wall_ns_per_legal_move = result.solve_wall_ns_per_legal_move,
        push_wall_secs = result.push_wall_secs,
        fixpoint_secs = result.fixpoint_secs,
        audit = opt_u64(result.audit),
        cross_check = opt_u64(result.cross_check),
        validation_label = result.validation_label,
        table = table,
    )
}

fn json_report(result: &RunResult<'_>) -> String {
    let table_path = result
        .table_dump
        .as_ref()
        .map_or("null".to_string(), |dump| json_string_value(&dump.path));
    let table_records = result
        .table_dump
        .as_ref()
        .map_or("null".to_string(), |dump| dump.records.to_string());
    let table_bytes = result
        .table_dump
        .as_ref()
        .map_or("null".to_string(), |dump| dump.bytes.to_string());
    let table_record_bytes = result
        .table_dump
        .as_ref()
        .map_or("null".to_string(), |dump| dump.record_bytes.to_string());
    let table_format = result
        .table_dump
        .as_ref()
        .map_or("null".to_string(), |dump| json_string_value(dump.format));
    format!(
        concat!(
            "{{\n",
            "  \"rung\": \"{rung}\",\n",
            "  \"start\": \"{start}\",\n",
            "  \"positions_reachable_canonical\": {positions},\n",
            "  \"start_value\": {start_value},\n",
            "  \"verdict\": \"{verdict}\",\n",
            "  \"wins\": {wins},\n",
            "  \"losses\": {losses},\n",
            "  \"draws\": {draws},\n",
            "  \"max_dtm\": {max_dtm},\n",
            "  \"total_legal_moves\": {total_moves},\n",
            "  \"avg_branching\": {avg_branching:.6},\n",
            "  \"propagation_edges\": {propagation_edges},\n",
            "  \"propagation_ns_per_edge\": {propagation_ns_per_edge:.3},\n",
            "  \"solve_wall_ns_per_legal_move\": {solve_wall_ns_per_legal_move:.3},\n",
            "  \"push_wall_secs\": {push_wall_secs:.6},\n",
            "  \"bfs_propagation_secs\": {fixpoint_secs:.6},\n",
            "  \"audit_bad_positions\": {audit},\n",
            "  \"push_vs_pull_mismatches\": {cross_check},\n",
            "  \"validation_label\": \"{validation_label}\",\n",
            "  \"table_dump_path\": {table_path},\n",
            "  \"table_dump_records\": {table_records},\n",
            "  \"table_dump_bytes\": {table_bytes},\n",
            "  \"table_record_bytes\": {table_record_bytes},\n",
            "  \"table_format\": {table_format}\n",
            "}}\n",
        ),
        rung = json_string(result.rung),
        start = json_string(&result.start),
        positions = result.positions,
        start_value = result.start_value,
        verdict = json_string(&result.verdict),
        wins = result.wins,
        losses = result.losses,
        draws = result.draws,
        max_dtm = result.max_dtm,
        total_moves = result.total_moves,
        avg_branching = result.avg_branching,
        propagation_edges = result.propagation_edges,
        propagation_ns_per_edge = result.propagation_ns_per_edge,
        solve_wall_ns_per_legal_move = result.solve_wall_ns_per_legal_move,
        push_wall_secs = result.push_wall_secs,
        fixpoint_secs = result.fixpoint_secs,
        audit = opt_json_u64(result.audit),
        cross_check = opt_json_u64(result.cross_check),
        validation_label = json_string(&result.validation_label),
        table_path = table_path,
        table_records = table_records,
        table_bytes = table_bytes,
        table_record_bytes = table_record_bytes,
        table_format = table_format,
    )
}

fn opt_u64(value: Option<u64>) -> String {
    value.map_or("skipped".to_string(), |v| v.to_string())
}

fn opt_json_u64(value: Option<u64>) -> String {
    value.map_or("null".to_string(), |v| v.to_string())
}

fn json_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn json_string_value(value: &str) -> String {
    format!("\"{}\"", json_string(value))
}
