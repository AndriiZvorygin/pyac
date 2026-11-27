//! GPU parity is a design constraint on the core, so its crates must stay here.
//!
//! `pyac-vliw-core` is `#![no_std]` precisely so the same source runs on the CPU
//! and on both GPU targets. If the kernels ever move out — or stop depending on
//! the core — parity becomes a claim about two implementations instead of a
//! property of one. These checks are cheap and they catch that drift.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("crate lives two levels below the workspace root")
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

#[test]
fn both_gpu_kernels_build_from_the_shared_core() {
    // A kernel that stopped importing the core would be a second implementation.
    for (crate_path, label) in [
        ("crates/gpu/shader/src/lib.rs", "SPIR-V"),
        ("crates/gpu/cuda-kernel/src/lib.rs", "CUDA"),
    ] {
        let source = read(crate_path);
        assert!(
            source.contains("pyac_vliw_core"),
            "the {label} kernel must build from pyac-vliw-core, not a copy"
        );
    }
}

#[test]
fn the_core_stays_no_std_so_it_can_compile_for_a_device() {
    // Dropping no_std would silently make the core host-only: it would still
    // build for the CPU and stop being compilable for spirv/nvptx.
    let source = read("crates/pyac-vliw-core/src/semantics_runtime.rs");
    assert!(
        source.starts_with("#![no_std]"),
        "pyac-vliw-core must remain #![no_std]; the GPU targets depend on it"
    );
}

#[test]
fn the_core_declares_the_spirv_target_dependency() {
    let manifest = read("crates/pyac-vliw-core/Cargo.toml");
    assert!(
        manifest.contains("target_arch = \"spirv\""),
        "the core must keep its spirv target dependency, or the SPIR-V kernel stops building"
    );
    assert!(
        manifest.contains("spirv-std"),
        "the core must keep spirv-std for the SPIR-V target"
    );
}

#[test]
fn the_worker_is_a_workspace_member_so_its_binary_is_built() {
    // The GPU worker hosts the CUDA and Vulkan runtimes. It must be a member:
    // excluded, `cargo build` would skip it and the parity probe would have
    // nothing to talk to.
    let manifest = read("Cargo.toml");
    assert!(
        manifest.contains("crates/gpu/worker"),
        "the GPU worker must be a workspace member"
    );
    // The kernels cannot be members: they are `no_std` and only build for their
    // own targets, so a host build would try to compile them for x86 and fail.
    assert!(
        manifest.contains("exclude") && manifest.contains("crates/gpu/shader"),
        "the shader and CUDA kernels must stay excluded from a host build"
    );
}

#[test]
fn the_cuda_build_compiles_artifacts_before_service_start() {
    // A production service must load compiled artifacts; it may not compile at
    // startup, and readiness is capped at 60 seconds.
    let dockerfile = read("container/pyac-vliw-runtime/Dockerfile.cuda");
    assert!(
        dockerfile.contains("ptxas"),
        "the CUDA image must assemble the cubin at build time"
    );
    assert!(
        dockerfile.contains("cubin.sha256"),
        "the image must record the cubin digest it built"
    );
    let compose = read("container/pyac-vliw-runtime/compose.yaml");
    assert!(
        !compose.contains("cargo build") && !compose.contains("nvcc"),
        "the runtime service must not compile anything at startup"
    );
}
