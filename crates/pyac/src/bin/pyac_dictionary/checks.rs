//! Checking a volume, and measuring it.

use super::*;

pub(super) fn verify(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 2) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let verify_start = Instant::now();
    let keys = reader.canonical_keys()?;
    let mut failures: Vec<String> = Vec::new();
    for key in &keys {
        match reader.read_entry(key) {
            Ok(codelets) if !codelets.is_empty() => {}
            Ok(_) => failures.push(format!("key {key:x?}: decoded as empty")),
            Err(error) => failures.push(format!("key {key:x?}: {error}")),
        }
    }
    let verify_ns = verify_start.elapsed().as_nanos();
    let verified = keys.len();
    Ok(json!({
        "command": "verify", "path": path,
        "entries": reader.entry_count(), "keys": verified,
        "failures": failures, "ok": failures.is_empty(),
        "volumeBytes": size, "openMs": open_ms, "verifyNs": verify_ns
    }))
}

pub(super) fn benchmark(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 6 && args[3] == "surface") {
        return Err(usage().into());
    }
    let iterations = args[2].parse::<u64>()?;
    if iterations == 0 {
        return Err("benchmark iterations must be positive".into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    if let Some(reason) = benchmark_guard_reason(reader.profile_version()) {
        return Err(io::Error::other(reason).into());
    }
    for _ in 0..5 {
        std::hint::black_box(reader.lookup_surface(&args[4], &args[5])?);
    }
    let lookup_start = Instant::now();
    let mut match_count = 0usize;
    for _ in 0..iterations {
        match_count += std::hint::black_box(reader.lookup_surface(&args[4], &args[5])?).len();
    }
    let lookup_ns = lookup_start.elapsed().as_nanos();
    Ok(json!({
        "command": "benchmark", "path": path, "language": args[4], "surface": args[5],
        "iterations": iterations, "warmupIterations": 5,
        "matchesPerIteration": match_count / iterations as usize,
        "volumeBytes": size, "codeletCount": reader.codelet_count(),
        "entryCount": reader.entry_count(),
        "openMs": open_ms, "lookupTotalNs": lookup_ns,
        "lookupMeanNs": lookup_ns / u128::from(iterations),
        "lookupsPerSecond": (iterations as f64 * 1_000_000_000.0)
            / lookup_ns.max(1) as f64
    }))
}
