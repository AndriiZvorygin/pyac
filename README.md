# pyac

The VLIW implementation: a 16-bit-word instruction set, an execution runtime, and
the multilingual dictionary that lives inside it. Rust only.

This is the lean implementation. It bootstraps itself — it can compile its own
dictionary from source with no JavaScript, no `node`, and no generated tables.

## What is here

```
crates/pyac-vliw-core   the ISA
                        codelets, programlet execution, storage sections,
                        literal and vector encoding, the word codec
crates/pyac             the dictionary and runtime services
                        volume reader/writer, in-band indexes, source parser,
                        catalog parser, IWSP evaluator, RISC-V backend,
                        the pyac-dictionary CLI
crates/gpu/shader       SPIR-V compute kernel (Rust-GPU); builds, pipeline cache
                        unproven — see "GPU parity" below
crates/gpu/cuda-kernel  PTX/CUDA kernel
crates/gpu/worker       the GPU runtime service: wgpu + CUDA, HTTP execute API
container/              the CUDA image build (cubin compiled at build time)
dictionary/             the vocabulary: authored source, and how to build it
docs/                   the normative specifications this code implements
```

Five external dependencies: `serde`, `serde_json`, `unicode-normalization`, and
`spirv-std` (SPIR-V target only).

## Build and test

```sh
cargo build --release
cargo test --release
```

A clean clone is green with no dictionary volume present. The full-strength
dictionary gates skip until one exists; see `dictionary/README.md`.

## The dictionary self-hosts

```sh
# compile the authored source into the runtime volume
cargo run --release --bin pyac-dictionary -- \
  compile dictionary/reference.pya dictionary/reference.pyac

# what the volume says about a word
cargo run --release --bin pyac-dictionary -- surface-entries \
  dictionary/reference.pyac en "noun class"

# edit a volume without touching the source
cargo run --release --bin pyac-dictionary -- decode dictionary/reference.pyac > v.json
cargo run --release --bin pyac-dictionary -- build  dictionary/reference.pyac < v.json
```

`compile` takes 5.7 s for 64,598 entries. Its output is **byte-identical** to the
published volume, and `cargo test --test dictionary_compile` is the gate on that.

`decode` then `build` is also byte-identical, so the two cycles — regenerate from
source, or edit a volume in place — both round-trip exactly.

## The instruction set in one paragraph

A program is a flat stream of 16-word codelets, each 16-bit word. `PYACVLIW` is
the container magic; the header names the word layout and codelet count; the
payload is the codelets in order. Sentences are lowered into codelets by a
producer (the `pyash` repository's JavaScript implementation), and this crate
executes them: numbers, vectors, conditionals, programlets with routines and
frames, storage sections, and the index-word dispatch that resolves a verb by its
case and noun-class signature. Rust is the normative implementation — see
`docs/parity-gate.md`.

## Relation to `pyash`

`pyash` is the general-intelligence operating system project this was extracted
from. It keeps the language, the applications, the agents, and the runtime. This
repository is the compiled core: the ISA, its execution, and the vocabulary.

The authority story is already settled in the extracted `docs/parity-gate.md`: the
JavaScript interpreter is **historical and out of scope**, and the Rust
implementation is normative. This is not a fork of the reference implementation;
it is the reference implementation, separated from the tree it grew in.

Some examples and specifications still live in `pyash` and are linked rather than
copied.

## GPU parity is a constraint, not an add-on

`pyac-vliw-core` is `#![no_std]`, and both GPU kernels call into it directly:

| target | how |
|---|---|
| CPU | `pyac` executes core directly |
| CUDA / PTX | `crates/gpu/cuda-kernel` compiles core for `nvptx64-nvidia-cuda`, then `ptxas` assembles the cubin |
| SPIR-V | `crates/gpu/shader` compiles core via Rust-GPU, but see below |

Same source, three backends. Parity is therefore structural rather than a
checked claim: there is no second implementation to drift.

The **CUDA/PTX path is the proven one.** Build and prove it:

```sh
bash container/pyac-vliw-runtime/command/build.sh          # 90 s, needs Docker + ptxas
docker run -d --rm --name pyac-parity-test --network host --runtime nvidia --gpus all \
  -e PYAC_VLIW_RUNTIME_HOST=127.0.0.1 -e PYAC_VLIW_RUNTIME_PORT=18899 \
  -e PYAC_VLIW_WORKER_PATH=/service/pyac-vliw-gpu-worker \
  liberit/pyac-vliw-runtime:cuda-sm86-<cubin-sha-prefix>

bash container/pyac-vliw-runtime/command/parity.sh
```

On an RTX 3090 that reports `backend: rust-cuda-cubin`, loads the cubin in 0.13 s
at startup, and returns `1 plus 1 = 2` from the kernel. `parity.sh` exits non-zero
if the GPU disagrees.

The image compiles the cubin at **build** time and the runtime stage copies only
artifacts — service startup never compiles. Startup has a 60-second ceiling.

### Only the CUDA path is proven

The SPIR-V shader **compiles**, and `Dockerfile` validates its memory model, but
the Vulkan pipeline cache is not known to work and has not been demonstrated here.
Do not treat Vulkan as a supported backend on the strength of it building.

The cubin path is the working one: built, loaded, and exercised on an RTX 3090
returning the expected result.

## What is not here yet

`pyac-riscv` (RISC-V image build) and `pyac-browser` (wasm/wgpu bindings) still
live in `pyash/rust/`. Neither has a path dependency outside what is present, so
they move when they have something to be proven on.
