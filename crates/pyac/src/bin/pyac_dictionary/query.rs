//! Reading a volume: what it holds, and where.

use super::*;

/// Show a volume's metadata and section layout.
///
/// **examine**, Pyac key `hzi7n` (`pyash dict from pyac hzi7n`): *"consider in
/// detail and subject to analysis"*.
pub(super) fn examine(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 2) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let metadata = reader.metadata();
    Ok(json!({
        "command": "examine", "path": path, "volumeBytes": size,
        "codeletCount": reader.codelet_count(), "entryCount": reader.entry_count(),
        "metadata": {
            "kind": metadata.kind, "title": metadata.title,
            "revision": metadata.revision, "languages": metadata.languages,
            "normalizationPolicy": metadata.normalization_policy
        },
        "sections": reader.sections().iter().map(|section| json!({
            "role": section.role, "codeletStart": section.codelet_start,
            "codeletCount": section.codelet_count
        })).collect::<Vec<_>>(),
        "openMs": open_ms
    }))
}

pub(super) fn surface(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 4) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let lookup_start = Instant::now();
    let offsets = reader.lookup_surface(&args[2], &args[3])?;
    let lookup_ns = lookup_start.elapsed().as_nanos();
    Ok(json!({
        "command": "surface", "path": path, "language": args[2], "surface": args[3],
        "matches": offsets, "volumeBytes": size, "openMs": open_ms,
        "lookupNs": lookup_ns
    }))
}

pub(super) fn surface_entries(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 4) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let lookup_start = Instant::now();
    let offsets = reader.lookup_surface(&args[2], &args[3])?;
    let matches = offsets
    .iter()
    .map(|offset| {
        Ok(json!({
            "offset": offset,
            "codelets": reader.read_entry_at(*offset)?.iter().map(|codelet| codelet.to_vec()).collect::<Vec<_>>()
        }))
    })
    .collect::<Result<Vec<_>, pyac::dictionary::DictionaryReadError>>()?;
    let lookup_ns = lookup_start.elapsed().as_nanos();
    Ok(json!({
        "command": "surface-entries", "path": path, "language": args[2], "surface": args[3],
        "matches": matches, "volumeBytes": size, "openMs": open_ms,
        "lookupNs": lookup_ns
    }))
}

pub(super) fn source(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 5) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let lookup_start = Instant::now();
    let matches = reader.lookup_source(&args[2], &args[3], &args[4])?;
    let lookup_ns = lookup_start.elapsed().as_nanos();
    let matches = matches
        .iter()
        .map(|offset| json!({"offset": offset}))
        .collect::<Vec<_>>();
    Ok(json!({
        "command": "source", "path": path, "dataset": args[2], "release": args[3],
        "identifier": args[4], "matches": matches, "volumeBytes": size,
        "openMs": open_ms, "lookupNs": lookup_ns
    }))
}

pub(super) fn source_entries(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 5) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let lookup_start = Instant::now();
    let source_matches = reader.lookup_source(&args[2], &args[3], &args[4])?;
    let matches = source_matches
    .iter()
    .map(|offset| {
        Ok(json!({
            "offset": offset,
            "codelets": reader.read_entry_at(*offset)?.iter().map(|codelet| codelet.to_vec()).collect::<Vec<_>>()
        }))
    })
    .collect::<Result<Vec<_>, pyac::dictionary::DictionaryReadError>>()?;
    let lookup_ns = lookup_start.elapsed().as_nanos();
    Ok(json!({
        "command": "source-entries", "path": path, "dataset": args[2], "release": args[3],
        "identifier": args[4], "matches": matches, "volumeBytes": size,
        "openMs": open_ms, "lookupNs": lookup_ns
    }))
}

/// List every key with its English forms.
///
/// **export**, Pyac key `hfat sroc` (`pyash dict from pyac "hfat sroc"`):
/// *"transfer (electronic data) out of a database or document in a format that can
/// be used by other programs"*.
pub(super) fn export(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 2) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let scan_start = Instant::now();
    let entries = reader.entries_with_english()?;
    let scan_ns = scan_start.elapsed().as_nanos();
    let atoms = entries.iter().filter(|(key, _)| key.len() == 1).count();
    Ok(json!({
        "command": "export", "path": path,
        "entries": entries.iter().map(|(key, forms)| json!({
            "key": key,
            "words": forms,
        })).collect::<Vec<_>>(),
        "count": entries.len(),
        "atoms": atoms,
        "volumeBytes": size, "openMs": open_ms, "scanNs": scan_ns
    }))
}

/// Show one entry's codelets by canonical key.
///
/// **entry**, Pyac key `nyit lwat` (`pyash dict from pyac "nyit lwat"`): *"an item
/// inserted in a written record"*. Not `hjac` ("submitted for judgment").
pub(super) fn key(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    let command = args[0].as_str();
    if !(args.len() == 3) {
        return Err(usage().into());
    }
    let path = Path::new(&args[1]);
    let key = parse_key(&args[2]).map_err(io::Error::other)?;
    let size = fs::metadata(path)?.len();
    let open_start = Instant::now();
    let mut reader = open_reader(path)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let read_start = Instant::now();
    let codelets = if command == "entry" {
        reader.read_entry(&key)?
    } else {
        reader.read_definition(&key)?
    };
    let read_ns = read_start.elapsed().as_nanos();
    Ok(json!({
        "command": command, "path": path, "key": key, "codelets": codelets,
        "hasDefinition": reader.has_definition(&key)?,
        "volumeBytes": size, "openMs": open_ms, "readNs": read_ns
    }))
}
