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

## What is not here yet

`pyac-riscv` (RISC-V image build), `pyac-browser` (wasm/wgpu), and the GPU
kernels (SPIR-V shader, CUDA kernel, wgpu worker) exist in `pyash/rust/` and move
here once they have a device or toolchain to be proven on. They have no path
dependencies outside the two crates present.
