# AGENTS.md

Orientation for an agent working in this repository. Read this first. It exists
so a session here stays focused on the Rust implementation and does not wander
into the JavaScript project this was extracted from.

## What this repository is

The **compiled core of a GI-OS** — a general-intelligence operating system — and
nothing else. Concretely:

- the VLIW instruction set and its execution runtime
- the multilingual dictionary that lives inside the volume format
- the device kernels (CUDA) that run the same core
- the reference implementation that the rest of the system is measured against

Rust only. There is no JavaScript here by design, and none should be added.

## The one sentence that keeps you oriented

> **Rust is normative. A Pyac word's identity is its spelling. The dictionary
> volume is a build artifact.**

Everything below follows from those three.

## The three rules

### 1. Rust is normative

`docs/parity-gate.md` declares the authority map for the whole project: the Rust
implementation is the reference, and the historical JavaScript interpreter is
**out of scope and not a conformance target**. If you are comparing behaviour
against a JavaScript file in the `pyash` checkout, stop — that comparison is
backwards.

Corollary: `pyac-vliw-core` is `#![no_std]` on purpose. It compiles for the host,
for `nvptx64` (CUDA), and for `spirv`. Dropping `no_std`, or adding a `std`-only
dependency to the core, silently makes the device targets unbuildable. The
`gpu_parity_shape` test guards this.

### 2. A word's identity is its spelling

An entry in the dictionary may carry several references at once:

```
srim  (signature)
  wordnet     Open English WordNet 2025   n:06646237-n
  wiktionary  en_all_nopic_2026-08        signature
  sumo        WordNet 3.0 mapping         Signature
```

**None of those is the identity.** WordNet sense ids renumber between releases —
the volume already holds ids from two different WordNet generations — so they are
citations, never keys. The Pyac spelling is the key. `dictionary/SOURCES.md` has
the full account.

This also means: never build a lookup keyed on a sense id, a SUMO concept, or an
English surface. Those are attributes of an entry, not names for it.

### 3. The volume is a build artifact

`dictionary/reference.pyac` is 131 MB and reproducible. It is not committed. A
clean clone regenerates it:

```sh
cargo run --release --bin pyac-dictionary -- compile dictionary/shard dictionary/reference.pyac
```

The source ships as structural shards, not one file: it is 101.5 MB, over git's
100 MB per-file limit. `compile` takes either the shard directory or a single
`.en.pya`, so the command above and a whole-file build behave identically.

**The extension states the source form.** `.en.pya` is English-form Pyash — the
records here (`su name … ob text … be text ya`). Plain `.pya` is reserved for the
glyph Pyac form, which is the language itself. The splitter emits `.en.pya`; the
assembler accepts either on read. `docs/pyash-vliw-decode-and-dispatch.md` extends
the same convention to other languages (`.fr.pya`, `.zh.pya`).

That command is **byte-identical** to the published volume, and
`crates/pyac/tests/dictionary_compile.rs` is the gate on it. If it ever stops
being identical, that is a defect in the parser or encoder — not a reason to relax
the test.

That gate needs the published volume, so out of the box it skips. What runs
everywhere is `crates/pyac/tests/dictionary_selfhost.rs`: the source parses at
full scale, `compile` is deterministic, and the result survives decode→encode
unchanged. It is weaker than byte-identity against the shipped artifact, and it is
the reason "it self-hosts" is checkable on a fresh clone.

The dictionary tests skip with a message when no volume is present, so a clean
checkout is green. Do not turn those skips into failures; a clone has to build.

## Layout

```
crates/pyac-vliw-core/    the ISA. no_std. codelets, programlets, storage,
                          literals/vectors, the word codec.
crates/pyac/              dictionary reader/writer, source parser, shard
                          splitter/assembler, catalog parser, IWSP evaluator,
                          CLI.
crates/gpu/shader/        SPIR-V kernel. Builds; pipeline cache unproven.
crates/gpu/cuda-kernel/   PTX/CUDA kernel. The proven device path.
crates/gpu/worker/        GPU runtime service: HTTP execute API.
container/                image builds. See `container/pyac-vliw-runtime/`.
dictionary/               authored source, and how to build the volume.
docs/                     the normative specifications this code implements.
```

The shader and CUDA kernels are `exclude`d from the workspace, not members: they
only build for their own targets, so a host `cargo build` would try to compile
them for x86 and fail. The worker *is* a member, because its binaries are host
binaries.

## Commands you will actually use

```sh
cargo build --release
cargo test --release                       # green with no volume present

# the dictionary cycle
cargo run --release --bin pyac-dictionary -- compile dictionary/shard /tmp/v.pyac
cargo run --release --bin pyac-dictionary -- verify  /tmp/v.pyac
cargo run --release --bin pyac-dictionary -- decode  /tmp/v.pyac /tmp/v.en.pya
cargo run --release --bin pyac-dictionary -- surface-entries /tmp/v.pyac en "signature"

# with the real volume, run the full-strength gates
PYAC_REFERENCE_VOLUME=dictionary/reference.pyac cargo test --release
```

`compile` (source → volume) and `decode` → `compile` (volume → source → volume)
are **both** byte-identical to the input. Two independent round trips; a change
that breaks either is a real defect. There is no JSON form of the dictionary.

## The device path

Only the CUDA cubin is proven. It needs Docker and `ptxas`:

```sh
bash container/pyac-vliw-runtime/command/build.sh        # ~130 s, emits CUBIN_SHA256
# run the image, then:
bash container/pyac-vliw-runtime/command/parity.sh       # exits non-zero if GPU ≠ CPU
```

Verified on an RTX 3090: loads the cubin in 0.13 s at startup, `1 plus 1` returns
`2` from the kernel. **Vulkan is not a supported backend** — the shader compiles
and its memory model is validated, but the pipeline cache does not work and has
never been demonstrated. Do not claim otherwise.

Production rule that applies to the image: compile artifacts at **build** time,
never at service startup, and keep startup under 60 seconds.

## Where things are specified

| question | document |
|---|---|
| the instruction set | `docs/27-pyash-vliw-isa.md` |
| the volume format | `docs/pyac-vliw-dictionary.md` |
| the pack + catalog pair | `docs/pyac-vliw-database.md` |
| decoding and dispatch | `docs/pyash-vliw-decode-and-dispatch.md` |
| which implementation is authoritative | `docs/parity-gate.md` |
| where the vocabulary's evidence comes from | `dictionary/SOURCES.md` |
| how to work the vocabulary (for agents) | `skills/pyac-vocabulary/SKILL.md` |
| what the GUARD self-improvement paper means here | `docs/guard-integration.md` |

`skills/` is plain Markdown in the repository, not a plugin format. Read
`skills/README.md` for the index; it works under any harness, or by hand.

Start with the ISA. It is normative and everything else follows from it.

## Running checks without burning the context

The full suite takes about four minutes and prints roughly 40 KB. Most of that is
noise, and an agent that reads it pays for every byte.

```sh
cargo test --release -p pyac-vliw-core       # 0.6 s, 17 KB — the core and the ISA
cargo test --release -q                      # 10 KB instead of 40 KB
cargo test --release --test dictionary_pyac  # one gate, not forty
```

Gate the expensive ones only when they can fail: the dictionary gates need
`PYAC_REFERENCE_VOLUME` set and skip otherwise, so a plain `cargo test` is not
running them at all. Say which gates you ran rather than implying the full suite.

`decode` produces about 100 MB. Pass an output path
(`decode <volume> out.en.pya`); with no path it writes to stdout, so
`decode … > file` is correct but `decode … | head` is not — it fills the
transcript with half a vocabulary.

## Traps that have already cost time

- **Sense ids are not keys.** See rule 2. An `n:…` id moves between WordNet
  releases; a spelling does not.
- **A keyword between two lowercase words is usually a bad bulk rename.** Names
  like `grammatical_case` and `noun_class` are live vocabulary; do not "fix" them.
- **The source writes case-then-value; the volume writes value-then-case.** Reading
  one as the other parses nothing and reports nothing.
- **`ve` runs are decimal first.** `10590` is a number, not an oversized hex
  literal. `0x`-prefixed values are hex.
- **A module body in `src/bin/` is not a binary.** `pack_validation.rs` is
  included by `vliw_runtime.rs` through `#[path]`; hence `autobins = false`.
- **Name identifiers in English.** Pyac spelling belongs in a doc comment. A
  field called `name` invites putting the noun class (`gi`) into a documentation
  slot; the field is `tag_label`.
- **Every English word that names a vocabulary concept carries its Pyac compound
  at first use in the file.** One comment, next to where the word first appears:

  ```rust
  /// **examine**, Pyac key `hzi7n` (`pyash dict from pyac hzi7n`): *"consider in
  /// detail and subject to analysis"*.
  pub fn examine(...)
  ```

  A word with no entry is not a name yet. `render` was chosen by eye; the volume
  carries 11 senses of it and none is the operation, so it is now `decode`
  (`hkom tyac`, *"convert code into ordinary language"*). Check a candidate with
  `vocab_suggest` in the `pyash` checkout, and when you want the compound that
  *means* a thing rather than the atom near it, search **glosses**, not surfaces.
- **`cargo test` does not build benches or examples.** `cargo build --release
  --all-targets` and `cargo bench -p pyac --no-run` are separate checks, and a
  renamed field will sit broken in a bench for a long time otherwise.

## What not to do

- **Do not reach into the `pyash` checkout to "fix" this repository.** It is a
  sibling, not a dependency. If something is missing here, add it here.
- **Do not add JavaScript.** Not a helper script, not a test harness. The point of
  the extraction is that the core is Rust and stays Rust.
- **Do not vendor a corpus.** References are citations; `dictionary/SOURCES.md`
  explains resolution. A dataset that requires downloading bytes to build does not
  belong.
- **Do not relax a byte-identity gate** to make a change pass. Both round trips
  exist to catch silent corruption; an approximate match would hide exactly the
  defect class they were written for.
- **Do not widen the verb or crate surface without a parity target.** More code is
  not more capability.

## Scope discipline

Do the work that was asked, in this repository. If a prerequisite turns out to be
missing and lives in `pyash`, say so and stop rather than importing a subtree.
The extraction is deliberate and narrow; keeping it narrow is the work.

## Red, then green

Write the failing test before the implementation. Reading the source and reasoning
about what it "should" do produces confident, wrong conclusions; running it does
not. When you are unsure whether something is possible, the cheapest experiment is
a `#[test]` or a throwaway example that calls the real API and prints the real
error — not another pass over the code.

The order is: state the claim as an assertion, run it, watch it fail for the
reason you expect, then make it pass. A test that has never been red has not been
shown to test anything. If the failure is not the one you predicted, the
prediction was wrong and that is the finding.

## Commit when a change is done

A major change is not finished until it is committed. Leaving a finished change
uncommitted in the working tree means the next session sees two unrelated changes
mixed together and cannot tell which tests belong to which. After a significant
change that passes its gates — a format change, a rename that touches many files,
a doc reconciliation — commit it with a message that states the *why*, and keep
unrelated changes in separate commits.

Before committing a change that touches the ISA, the volume format, or the word
codec, run the full suite with a real volume:

```sh
cargo test --release
PYAC_REFERENCE_VOLUME=dictionary/reference.pyac cargo test --release
```
