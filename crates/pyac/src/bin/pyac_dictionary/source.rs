//! Compiling a volume, and decoding it back to source text.

use super::*;

pub(super) fn compile(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 3) {
        return Err(usage().into());
    }
    // The bootstrap: source in, volume out, no intermediate JSON and no
    // JavaScript. `build` remains for scripts that want to edit a request.
    //
    // The first argument is either a single `.en.pya` file or a shard
    // directory: a directory is assembled from its manifest first, so one
    // code path parses both.
    let source_path = Path::new(&args[1]);
    let text = if source_path.is_dir() {
        assemble_shard_directory(source_path)?
    } else {
        fs::read_to_string(source_path)?
    };
    let parse_started = Instant::now();
    let source = parse_source(&text)
        .map_err(|error| io::Error::other(format!("{}: {error}", error.code())))?;
    let parse_ms = parse_started.elapsed().as_secs_f64() * 1000.0;
    let entry_count = source.entries.len();
    let volume = pyac::dictionary::source_to_volume(source)?;
    let encode_started = Instant::now();
    let encoded = encode_dictionary(&volume)?;
    let encode_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
    let write_started = Instant::now();
    atomic_write(Path::new(&args[2]), &encoded)?;
    let write_ms = write_started.elapsed().as_secs_f64() * 1000.0;
    Ok(json!({
        "command": "compile", "source": args[1], "path": args[2],
        "entries": entry_count, "volumeBytes": encoded.len(),
        "codeletCount": encoded.len().saturating_sub(32) / 32,
        "sourceBytes": text.len(),
        "parseMs": parse_ms, "encodeMs": encode_ms, "writeMs": write_ms
    }))
}

pub(super) fn shard(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    if !(args.len() == 3) {
        return Err(usage().into());
    }
    // Split the source along its dependency order so each piece fits
    // standard git. See `dictionary/shard.rs` for the grouping.
    let source_path = Path::new(&args[1]);
    let out_dir = Path::new(&args[2]);
    let text = fs::read_to_string(source_path)?;
    let layers = load_layer_index()?;
    let set = split_source_with_layers(&text, &layers)
        .map_err(|error| io::Error::other(format!("{}: {error}", error.code())))?;
    fs::create_dir_all(out_dir)?;
    let mut written = Vec::new();
    for (name, body) in &set.shards {
        // A shard name may carry a directory (`dispatch/<key>`), so its
        // parent is created before the write.
        let path = out_dir.join(pyac::dictionary::shard_file_name(name));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, body)?;
        written.push(json!({
            "name": name, "file": path, "bytes": body.len()
        }));
    }
    let manifest_path = out_dir.join(MANIFEST_FILE);
    fs::write(&manifest_path, &set.manifest)?;
    Ok(json!({
        "command": "shard", "source": args[1], "directory": args[2],
        "shards": written, "manifest": manifest_path,
        "manifestBytes": set.manifest.len()
    }))
}

pub(super) fn decode(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    // The inverse of `compile`: a volume back to source text. With one
    // operand the source goes to stdout and the report to stderr; with an
    // output path the source is written there, so a 101 MB decode never
    // lands in a transcript. `--glyph` selects the Pyac form; the default
    // is the English form `.en.pya` reads.
    let operands: Vec<&String> = args
        .iter()
        .filter(|argument| argument.as_str() != "--glyph")
        .collect();
    if operands.len() < 2 || operands.len() > 3 {
        return Err(io::Error::other(usage()).into());
    }
    let glyph = args.iter().any(|argument| argument == "--glyph");
    let path = Path::new(operands[1]);
    let volume = read_volume(File::open(path)?)?;
    let source = if glyph {
        pyac::dictionary::decode_pyac_source(&volume)?
    } else {
        pyac::dictionary::decode_source(&volume)?
    };
    match operands.get(2) {
        Some(output) => {
            atomic_write(Path::new(output), source.as_bytes())?;
        }
        None => {
            // stdout for the source, so `decode … > file` is correct.
            print!("{source}");
        }
    }
    Ok(json!({
        "command": "decode", "path": operands[1],
        "form": if glyph { "pya" } else { "en.pya" },
        "output": operands.get(2).copied(),
        "entries": volume.entries.len(),
        "bytes": source.len(),
    }))
}
