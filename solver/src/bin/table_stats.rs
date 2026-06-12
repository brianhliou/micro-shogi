//! Scan a dumped Micro Shogi table and report value / DTM distributions.
//!
//! Usage:
//!
//!   cargo run --release --bin table_stats -- research/runs/kpg/kpg.table.bin

use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{self, BufReader, ErrorKind, Read};
use std::path::PathBuf;

const MAGIC: &[u8; 8] = b"MSHTB001";
const HEADER_BYTES: usize = 20;
const RECORD_BYTES: usize = 20;
const CHUNK_RECORDS: usize = 1 << 20;

fn main() {
    if let Err(err) = run() {
        eprintln!("table_stats: {err}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let path = parse_path()?;
    let file = File::open(&path)?;
    let mut reader = BufReader::with_capacity(32 * 1024 * 1024, file);

    let mut header = [0u8; HEADER_BYTES];
    reader.read_exact(&mut header)?;
    if &header[..MAGIC.len()] != MAGIC {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "table magic was not MSHTB001",
        ));
    }
    let records = u64::from_le_bytes(header[8..16].try_into().unwrap());
    let record_bytes = u32::from_le_bytes(header[16..20].try_into().unwrap());
    if record_bytes as usize != RECORD_BYTES {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("unsupported record size: {record_bytes}"),
        ));
    }

    let mut signed = BTreeMap::<i32, u64>::new();
    let mut dtm_wins = BTreeMap::<i32, u64>::new();
    let mut dtm_losses = BTreeMap::<i32, u64>::new();
    let mut decided_dtm = BTreeMap::<i32, u64>::new();
    let mut wins = 0u64;
    let mut losses = 0u64;
    let mut draws = 0u64;
    let mut win_dtm_sum = 0u128;
    let mut loss_dtm_sum = 0u128;
    let mut decided_dtm_sum = 0u128;
    let mut max_win_dtm = 0i32;
    let mut max_loss_dtm = 0i32;
    let mut max_abs_dtm = 0i32;

    let mut buf = vec![0u8; CHUNK_RECORDS * RECORD_BYTES];
    let mut remaining = records;
    while remaining > 0 {
        let chunk_records = remaining.min(CHUNK_RECORDS as u64) as usize;
        let chunk_bytes = chunk_records * RECORD_BYTES;
        reader.read_exact(&mut buf[..chunk_bytes])?;
        for record in buf[..chunk_bytes].chunks_exact(RECORD_BYTES) {
            let value = i32::from_le_bytes(record[16..20].try_into().unwrap());
            *signed.entry(value).or_default() += 1;
            if value > 0 {
                let dtm = value;
                wins += 1;
                win_dtm_sum += dtm as u128;
                decided_dtm_sum += dtm as u128;
                max_win_dtm = max_win_dtm.max(dtm);
                max_abs_dtm = max_abs_dtm.max(dtm);
                *dtm_wins.entry(dtm).or_default() += 1;
                *decided_dtm.entry(dtm).or_default() += 1;
            } else if value < 0 {
                let dtm = -value;
                losses += 1;
                loss_dtm_sum += dtm as u128;
                decided_dtm_sum += dtm as u128;
                max_loss_dtm = max_loss_dtm.max(dtm);
                max_abs_dtm = max_abs_dtm.max(dtm);
                *dtm_losses.entry(dtm).or_default() += 1;
                *decided_dtm.entry(dtm).or_default() += 1;
            } else {
                draws += 1;
            }
        }
        remaining -= chunk_records as u64;
    }

    let scanned = wins + losses + draws;
    if scanned != records {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("scanned {scanned} records, expected {records}"),
        ));
    }

    let decided = wins + losses;
    println!("# Micro Shogi table stats");
    println!("path = {}", path.display());
    println!("records = {records}");
    println!("record_bytes = {record_bytes}");
    println!("wins = {wins}");
    println!("losses = {losses}");
    println!("draws = {draws}");
    println!("decided = {decided}");
    println!("max_win_dtm = {max_win_dtm}");
    println!("max_loss_dtm = {max_loss_dtm}");
    println!("max_abs_dtm = {max_abs_dtm}");
    println!("mean_win_dtm = {:.6}", mean(win_dtm_sum, wins));
    println!("mean_loss_dtm = {:.6}", mean(loss_dtm_sum, losses));
    println!("mean_decided_dtm = {:.6}", mean(decided_dtm_sum, decided));
    println!(
        "p50_decided_dtm = {}",
        percentile(&decided_dtm, decided, 50)
    );
    println!(
        "p90_decided_dtm = {}",
        percentile(&decided_dtm, decided, 90)
    );
    println!(
        "p99_decided_dtm = {}",
        percentile(&decided_dtm, decided, 99)
    );
    println!(
        "p999_decided_dtm = {}",
        percentile_per_mille(&decided_dtm, decided, 999)
    );

    println!();
    println!("## signed value histogram");
    println!("value,count");
    for (value, count) in &signed {
        println!("{value},{count}");
    }

    println!();
    println!("## absolute DTM by outcome");
    println!("dtm,wins,losses,decided");
    for dtm in 1..=max_abs_dtm {
        let w = dtm_wins.get(&dtm).copied().unwrap_or(0);
        let l = dtm_losses.get(&dtm).copied().unwrap_or(0);
        if w != 0 || l != 0 {
            println!("{dtm},{w},{l},{}", w + l);
        }
    }

    Ok(())
}

fn parse_path() -> io::Result<PathBuf> {
    let mut args = env::args_os().skip(1);
    match (args.next(), args.next()) {
        (Some(path), None) => Ok(PathBuf::from(path)),
        _ => Err(io::Error::new(
            ErrorKind::InvalidInput,
            "usage: table_stats PATH",
        )),
    }
}

fn mean(sum: u128, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    }
}

fn percentile(hist: &BTreeMap<i32, u64>, total: u64, percentile: u64) -> i32 {
    percentile_numerator(hist, total, percentile, 100)
}

fn percentile_per_mille(hist: &BTreeMap<i32, u64>, total: u64, per_mille: u64) -> i32 {
    percentile_numerator(hist, total, per_mille, 1000)
}

fn percentile_numerator(
    hist: &BTreeMap<i32, u64>,
    total: u64,
    numerator: u64,
    denominator: u64,
) -> i32 {
    if total == 0 {
        return 0;
    }
    let target = (total as u128 * numerator as u128).div_ceil(denominator as u128);
    let mut cumulative = 0u128;
    for (&dtm, &count) in hist {
        cumulative += count as u128;
        if cumulative >= target {
            return dtm;
        }
    }
    0
}
