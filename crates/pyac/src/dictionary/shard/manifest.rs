//! The manifest: one record per shard, with its digest and entry count.

use super::blocks::entry_blocks;
use super::errors::ShardError;
use std::collections::BTreeMap;

/// One content shard as the manifest records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardRecord {
    pub name: String,
    pub digest: String,
    pub entries: usize,
    pub max_layer: u16,
}
/// A split source: the manifest text plus each content shard's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardSet {
    pub manifest: String,
    pub shards: Vec<(String, String)>,
}
/// sha256 of the shard bytes, hex encoded, as the manifest records it.
pub fn shard_digest(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
/// Assemble a manifest and its content shards back into one source document.
///
/// Verifies every digest and entry count, and that no compound precedes a
/// component. Any mismatch is an error rather than a best-effort assembly, because
/// a shard set that has drifted must not compile into a plausible-looking volume.
pub fn assemble_shards(
    manifest: &str,
    contents: &BTreeMap<String, String>,
) -> Result<String, ShardError> {
    let records = parse_manifest(manifest)?;
    if records.is_empty() {
        return Err(ShardError::MissingManifest);
    }

    // Layers must ascend, so a compound's dependencies are already written. The
    // dispatch shard carries no vocabulary and declares layer 0 by convention, so
    // it is exempt from the ascent — it annotates entries that already exist.
    let mut previous_layer = 0u16;
    for record in &records {
        if record.name.starts_with("dispatch/") {
            continue;
        }
        if record.max_layer < previous_layer {
            return Err(ShardError::LayerOrder {
                previous: previous_layer,
                next: record.max_layer,
            });
        }
        previous_layer = record.max_layer;
    }

    let header = manifest
        .lines()
        .filter(|line| line.trim().starts_with("su name lyin kwon "))
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = String::new();
    if !header.is_empty() {
        out.push_str(&header);
        out.push('\n');
    }

    let mut seen: BTreeMap<String, ()> = BTreeMap::new();
    for record in &records {
        let Some(body) = contents.get(&record.name) else {
            return Err(ShardError::MissingShard {
                name: record.name.clone(),
            });
        };
        let digest = shard_digest(body);
        if digest != record.digest {
            return Err(ShardError::DigestMismatch {
                name: record.name.clone(),
                expected: record.digest.clone(),
                actual: digest,
            });
        }
        // A vocabulary shard declares its entry count; the dispatch shard
        // declares how many signatures it carries, because its records annotate
        // entries declared elsewhere.
        let actual = if record.name.starts_with("dispatch/") {
            body.matches(" at text \"operation-signature\" be text ya")
                .count()
        } else {
            entry_blocks(body).len()
        };
        if actual != record.entries {
            return Err(ShardError::CountMismatch {
                name: record.name.clone(),
                expected: record.entries,
                actual,
            });
        }
        if record.name.starts_with("dispatch/") {
            // Dispatch names entries that must already exist, but nothing depends
            // on dispatch, so it needs no ordering beyond coming last.
            out.push_str(body);
            continue;
        }
        let blocks = entry_blocks(body);
        // Every component must already be present. This is the same rule the
        // volume writer enforces; catching it here names the shard at fault.
        for block in &blocks {
            for reference in block.references() {
                if !seen.contains_key(&reference) {
                    return Err(ShardError::DependencyOrder {
                        key: block.key.clone(),
                        missing: reference,
                    });
                }
            }
        }
        for block in &blocks {
            seen.insert(block.key.clone(), ());
        }
        out.push_str(body);
    }
    out.push_str("prah\n");
    Ok(out)
}
/// Read a manifest's shard rows.
pub fn parse_manifest(manifest: &str) -> Result<Vec<ShardRecord>, ShardError> {
    let mut records = Vec::new();
    let mut saw_header = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() || line == "prah" {
            continue;
        }
        if line.starts_with("su name ") && line.ends_with(" be map def") {
            saw_header = true;
            continue;
        }
        if !line.starts_with("su text ") {
            continue;
        }
        let Some(rest) = line.strip_prefix("su text ") else {
            continue;
        };
        let Some((name_token, tail)) = rest.split_once(" ob text ") else {
            continue;
        };
        let Some(payload) = tail.strip_suffix(" ya") else {
            continue;
        };
        let name: String =
            serde_json::from_str(name_token).map_err(|_| ShardError::MalformedRow {
                name: name_token.to_owned(),
            })?;
        if name == "format" {
            continue;
        }
        let raw: String = serde_json::from_str(payload)
            .map_err(|_| ShardError::MalformedRow { name: name.clone() })?;
        let row: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|_| ShardError::MalformedRow { name: name.clone() })?;
        records.push(ShardRecord {
            name: row
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned(),
            digest: row
                .get("sha256")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned(),
            entries: row
                .get("entries")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default() as usize,
            max_layer: row
                .get("maxLayer")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default() as u16,
        });
    }
    if !saw_header {
        return Err(ShardError::MissingManifest);
    }
    Ok(records)
}
