use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pyac::dictionary::{
    DictionaryFileReader, LAYERS, LayerIndex, MANIFEST_FILE, assemble_shards, decode_dictionary,
    encode_dictionary, parse_manifest, parse_source, split_source_with_layers,
};
use serde_json::{Value, json};

#[path = "pyac_dictionary/checks.rs"]
mod checks;
#[path = "pyac_dictionary/query.rs"]
mod query;
#[path = "pyac_dictionary/source.rs"]
mod source;

fn usage() -> &'static str {
    "usage:\n  pyac-dictionary compile <source.en.pya|shard-dir> <volume.pyac>  # source in, volume out: the whole cycle.\n  pyac-dictionary inspect <volume.pyac>\n  pyac-dictionary surface <volume.pyac> <language> <surface>\n  pyac-dictionary surface-entries <volume.pyac> <language> <surface>\n  pyac-dictionary source <volume.pyac> <dataset> <release> <identifier>\n  pyac-dictionary source-entries <volume.pyac> <dataset> <release> <identifier>\n  pyac-dictionary key <volume.pyac> <hnuc[,hnuc...]>\n  pyac-dictionary entry <volume.pyac> <hnuc[,hnuc...]>\n  pyac-dictionary render <volume.pyac> [out.pya] [--glyph]  # the inverse of compile; stdout if no path.\n  pyac-dictionary verify <volume.pyac>  # every canonical key must read back.\n  pyac-dictionary export <volume.pyac>  # every key with its English forms.\n  pyac-dictionary benchmark <volume.pyac> <iterations> surface <language> <surface>"
}

fn binary_bytes() -> io::Result<u64> {
    Ok(fs::metadata(env::current_exe()?)?.len())
}

fn parse_key(value: &str) -> Result<Vec<u16>, String> {
    let key = value
        .split(',')
        .map(|word| {
            let word = word.trim().trim_start_matches("0x");
            u16::from_str_radix(word, 16).map_err(|_| format!("invalid HNUC word: {word}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if key.is_empty() {
        return Err("Pyac key must contain at least one HNUC word".into());
    }
    Ok(key)
}
/// Read a shard directory: manifest plus every shard it names, assembled.
fn assemble_shard_directory(directory: &Path) -> io::Result<String> {
    let manifest_path = directory.join(MANIFEST_FILE);
    let manifest = fs::read_to_string(&manifest_path).map_err(|error| {
        io::Error::other(format!(
            "shard directory {} has no {MANIFEST_FILE}: {error}",
            directory.display()
        ))
    })?;
    let records = parse_manifest(&manifest)
        .map_err(|error| io::Error::other(format!("{}: {error}", error.code())))?;
    let mut contents = std::collections::BTreeMap::new();
    for record in records {
        let body = pyac::dictionary::shard_candidates(&record.name)
            .into_iter()
            .find_map(|file| fs::read_to_string(directory.join(file)).ok())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "shard {} is named by the manifest but neither .{} nor .{} exists",
                        record.name,
                        pyac::dictionary::EN_SOURCE_EXT,
                        pyac::dictionary::PYAC_SOURCE_EXT
                    ),
                )
            })?;
        contents.insert(record.name.clone(), body);
    }
    assemble_shards(&manifest, &contents)
        .map_err(|error| io::Error::other(format!("{}: {error}", error.code())))
}

/// The glyph layer lists, for deciding which shard an entry belongs to.
///
/// They live beside the vocabulary in the `pyash` checkout, not here, so the path
/// is configured rather than assumed. Without them the split still works, it just
/// puts every compound in one shard.
fn load_layer_index() -> io::Result<LayerIndex> {
    let Ok(directory) = env::var("PYAC_LAYER_LISTS") else {
        return Ok(LayerIndex::default());
    };
    let mut lists = Vec::new();
    for layer in LAYERS {
        let path = Path::new(&directory).join(format!("{layer}GlyphWordList.txt"));
        let Ok(text) = fs::read_to_string(&path) else {
            return Err(io::Error::other(format!(
                "PYAC_LAYER_LISTS is set but {} is not readable",
                path.display()
            )));
        };
        lists.push((layer, text));
    }
    Ok(LayerIndex::from_lists(lists))
}

/// Turn a parsed source directly into a volume.
///
/// `compile` reads the `.en.pya` source, parses it, and calls this. There is no
/// intermediate JSON: the source is the intermediate form, and it round-trips.
fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temporary = PathBuf::from(path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("pyac");
    temporary.set_extension(format!("{extension}.tmp-{}", std::process::id()));
    let result = (|| {
        let mut file = File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// The throughput benchmark measures the direct-file indexes of a v4 volume,
/// so every older profile is refused instead of measured with compatibility
/// scans.
fn benchmark_guard_reason(profile_version: &str) -> Option<&'static str> {
    if profile_version == "pyac-vliw-dictionary/4" {
        return None;
    }
    Some(
        "dictionary throughput benchmark requires v4 direct-file indexes; v2/v3 use compatibility scans",
    )
}

fn open_reader(path: &Path) -> Result<DictionaryFileReader<File>, Box<dyn std::error::Error>> {
    Ok(DictionaryFileReader::open(File::open(path)?)?)
}

fn execute(args: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
    let binary_size = binary_bytes()?;
    let whole_start = Instant::now();
    let command = args.first().map(String::as_str).unwrap_or("");
    let output = match command {
        "compile" => source::compile(args)?,
        "shard" => source::shard(args)?,
        "render" => source::render(args)?,
        "inspect" => query::inspect(args)?,
        "surface" => query::surface(args)?,
        "surface-entries" => query::surface_entries(args)?,
        "source" => query::source(args)?,
        "source-entries" => query::source_entries(args)?,
        "key" | "entry" => query::key(args)?,
        "export" => query::export(args)?,
        "verify" => checks::verify(args)?,
        "benchmark" => checks::benchmark(args)?,
        _ => return Err(usage().into()),
    };
    let mut result = output;
    if let Some(object) = result.as_object_mut() {
        object.insert("executableBytes".into(), json!(binary_size));
        object.insert(
            "totalMs".into(),
            json!(whole_start.elapsed().as_secs_f64() * 1000.0),
        );
    }
    Ok(result)
}

fn main() {
    match execute(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(result) => {
            // `verify` reports its verdict in the body, so its exit status has to
            // carry the same answer for a shell or CI step to act on it.
            let failed_verify = result.get("ok").and_then(|value| value.as_bool()) == Some(false);
            println!("{result}");
            if failed_verify {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("pyac-dictionary: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::benchmark_guard_reason;

    #[test]
    fn the_benchmark_guard_refuses_every_profile_before_v4() {
        for profile in [
            "pyac-vliw-dictionary/2",
            "pyac-vliw-dictionary/3",
            "pyac-vliw-dictionary/1",
        ] {
            let reason = benchmark_guard_reason(profile).expect("legacy profiles are refused");
            assert!(reason.contains("requires v4 direct-file indexes"));
        }
        assert!(benchmark_guard_reason("pyac-vliw-dictionary/4").is_none());
    }
}
