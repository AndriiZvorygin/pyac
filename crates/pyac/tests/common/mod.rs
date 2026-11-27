//! Where the reference dictionary volume lives, and how to cope when it is absent.
//!
//! The volume is a 131 MB build artifact, not source. A fresh clone has the
//! authored `dictionary/reference.pya` and can regenerate the volume with
//! `pyac-dictionary compile`; until it does, the full-strength gates cannot run.
//!
//! They therefore *skip with a message* rather than failing. Failing would make a
//! clean checkout unbuildable for no reason, and the gates would stop being run at
//! all. The same shape is already used for the word-codec corpus in
//! `pyac-vliw-core/tests/word_code_diff.rs`.
//!
//! Resolution order:
//!   1. `PYAC_REFERENCE_VOLUME`, so CI or a working copy elsewhere can point at it
//!   2. `dictionary/reference.pyac` beside the workspace root
//!   3. the historical location inside the `pyash` checkout, for local use

use std::path::PathBuf;

/// The volume path, or `None` when no candidate exists.
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

    candidates.into_iter().find(|path| path.is_file())
}

/// The volume path, or `None` with the reason printed.
pub fn reference_volume_or_skip() -> Option<PathBuf> {
    let path = reference_volume();
    if path.is_none() {
        eprintln!(
            "skipping: no reference volume. Set PYAC_REFERENCE_VOLUME, or run \
             `pyac-dictionary compile dictionary/reference.pya dictionary/reference.pyac` first."
        );
    }
    path
}
