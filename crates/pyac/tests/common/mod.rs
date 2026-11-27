//! Where the reference dictionary volume lives, and how to cope when it is absent.
//!
//! The volume is a 131 MB build artifact. It is either regenerated with
//! `pyac-dictionary compile` or fetched from LFS, so there are three states a
//! candidate can be in:
//!
//! 1. a real volume — a `PYACVLIW` header and megabytes of codelets
//! 2. an LFS pointer — 130-odd bytes of text, present on a clone before
//!    `git lfs pull`. It exists as a file but is not the volume.
//! 3. absent
//!
//! Only the first may be used. Treating a pointer as a volume is worse than
//! having nothing: the gates fail with a byte-difference panic, which reads as a
//! defect in the encoder rather than "you have not fetched the artifact yet".
//! That is the opposite of the skip-don't-fail policy this repository is built
//! around, so a candidate is validated before it is offered.
//!
//! Resolution order:
//!   1. `PYAC_REFERENCE_VOLUME`, so CI or a working copy elsewhere can point at it
//!   2. `dictionary/reference.pyac` beside the workspace root
//!   3. the historical location inside the `pyash` checkout, for local use

use std::io::Read;
use std::path::{Path, PathBuf};

/// The container magic every v4 volume begins with.
const PACK_MAGIC: &[u8; 8] = b"PYACVLIW";

/// Anything smaller than this is not a volume, whatever it claims.
///
/// The smallest real volume this repository builds is megabytes; anything in the
/// low hundreds of bytes is an LFS pointer or a stray file. The exact floor does
/// not matter, only that it is far above a pointer and far below a real volume.
const MIN_VOLUME_BYTES: u64 = 1024 * 1024;

/// Whether `path` holds an actual volume rather than a pointer or a stray file.
fn is_volume(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() < MIN_VOLUME_BYTES {
        return false;
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 8];
    match file.read_exact(&mut magic) {
        Ok(()) => &magic == PACK_MAGIC,
        Err(_) => false,
    }
}

/// The volume path, or `None` when no usable candidate exists.
///
/// A candidate that is an LFS pointer is skipped, not returned: see the module
/// note.
pub fn reference_volume() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(explicit) = std::env::var_os("PYAC_REFERENCE_VOLUME") {
        candidates.push(PathBuf::from(explicit));
    }

    // The workspace root is two levels above a crate directory.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from);
    if let Some(root) = &workspace {
        candidates.push(root.join("dictionary/reference.pyac"));
        // The sibling checkout this crate was extracted from, so local work
        // continues before the volume has been regenerated here.
        candidates.push(root.join("../pyash/program/library/dictionary/reference.pyac"));
    }

    let mut saw_pointer = false;
    for candidate in candidates {
        if is_volume(&candidate) {
            return Some(candidate);
        }
        if candidate.is_file() && !saw_pointer {
            saw_pointer = true;
        }
    }
    if saw_pointer {
        eprintln!(
            "note: a volume path exists but is not a volume — most likely an LFS \
             pointer. Run `git lfs pull`, or compile it from dictionary/shard/."
        );
    }
    None
}

/// The volume path, or `None` with the reason printed.
pub fn reference_volume_or_skip() -> Option<PathBuf> {
    let path = reference_volume();
    if path.is_none() {
        eprintln!(
            "skipping: no usable reference volume. Run `git lfs pull`, or \
             `pyac-dictionary compile dictionary/shard dictionary/reference.pyac`."
        );
    }
    path
}
