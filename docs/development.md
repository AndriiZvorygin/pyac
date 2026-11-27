# Development

How to work in this repository. `AGENTS.md` is the orientation; this is the
day-to-day.

## A working cycle

```sh
cargo build --release
cargo test --release

# regenerate the volume, then run the gates against it
cargo run --release --bin pyac-dictionary -- \
  compile dictionary/reference.pya dictionary/reference.pyac
PYAC_REFERENCE_VOLUME=dictionary/reference.pyac cargo test --release
```

`cargo test` alone is green without a volume — the dictionary gates skip. That is
intentional so a clone builds. When you are changing the parser, the encoder, or
the codec, run with `PYAC_REFERENCE_VOLUME` set; otherwise you are running fewer
checks than you think.

## The two byte-identity gates

These are the strongest checks in the tree. Both compare bytes; neither accepts
"close enough".

| gate | what it proves | test |
|---|---|---|
| `compile` → published volume | the source parser and encoder are exact inverses of what produced the shipped volume | `crates/pyac/tests/dictionary_compile.rs` |
| `decode` → `build` | reading a volume and writing it back loses nothing | `crates/pyac/tests/dictionary_roundtrip.rs` |

They are independent: one can pass while the other fails. A failure in the first
is a parser or encoder defect; a failure in the second is a decoder defect. Both
report the byte offset of the first difference so you can find the entry.

If you are tempted to loosen either, that is the signal you have a real bug. The
decoder once produced an empty-labelled, doubled argument list on 1,810 of 1,832
signatures while `verify` reported 64,596/64,596 healthy — because the verifier
and the decoder shared the same wrong reading. Byte comparison is what caught it.

## Cheap tests to run while iterating

```sh
cargo test --release -p pyac-vliw-core           # ~25 s, the ISA
cargo test --release -p pyac --test dictionary_volume
cargo test --release --test gpu_parity_shape      # runs in ms once compiled;
                                                  # it is source-text checks, so a
                                                  # rebuild dominates the wall time
```

## Where a change belongs

| you are changing | it goes in |
|---|---|
| codelet layout, execution, dispatch | `crates/pyac-vliw-core/src/semantics_runtime/` |
| the word codec (spelling ↔ HNUC) | `crates/pyac-vliw-core/src/semantics_runtime/word_code.rs` |
| volume read/write, indexes | `crates/pyac/src/dictionary/` |
| the `.pya` source grammar | `crates/pyac/src/dictionary/source.rs` |
| the shipped vocabulary | `dictionary/reference.pya` (then `compile`) |
| what a citation resolves to | `dictionary/SOURCES.md` |
| a device kernel | `crates/gpu/{shader,cuda-kernel}/` |

## The word codec is the identity

`word_code::encode(spelling) -> u16` and `decode(u16) -> spelling` are the identity
relation for the whole system. There is no table: the codec computes. Two
consequences worth internalising:

1. A spelling not accepted by `encode` is not a word. `noun`, `class`, `return`,
   and `value` are not valid Pyac spellings — that is why `noun class` and
   `return value` are compounds, not atoms.
2. `word_code_diff.rs` compares the codec against a reference corpus when one is
   present at `/tmp/pyac_words.tsv`. It skips otherwise. If you are touching the
   codec, generate that corpus and run it.

## Adding vocabulary

Vocabulary enters through the source and `compile`, never by editing the volume:

1. add rows to `dictionary/reference.pya` (see the record kinds in
   `crates/pyac/src/dictionary/source.rs`)
2. `compile`
3. `verify` — every key must read back
4. `PYAC_REFERENCE_VOLUME=… cargo test` — the byte gates must still pass

A new concept that neither WordNet nor Wiktionary defines gets an authored gloss
under the `pyash-curated` dataset, with a `source-link` beside it (the writer
requires the pair). `dictionary/SOURCES.md` explains why that dataset is not
presented as external evidence.

## The device path

CUDA is proven; Vulkan is not. See `AGENTS.md`. The build is Docker-driven
because it needs a pinned nightly with `rust-src` and `ptxas`:

```sh
bash container/pyac-vliw-runtime/command/build.sh
```

The image compiles the cubin at build time and the runtime stage copies artifacts
only. Keep it that way: service startup must not compile, and must stay under 60
seconds.

## Style

- English identifiers. A Pyac spelling belongs in a doc comment when it
  disambiguates a sense; transliterating it into the name (`GJAT_MAGNITUDE`)
  hides the meaning.
- Comments explain *why*, and say so when behaviour is non-obvious or was once
  wrong. A comment that restates the code is noise.
- Prefer deleting a path over keeping a compatibility shim. This tree is young
  enough that there is nothing to be compatible with.
- Two space indent, trailing commas, `cargo fmt` clean.
- Clippy is not installed on the pinned toolchain. If you add it
  (`rustup component add clippy --toolchain 1.99.1`), treat its warnings as real
  rather than suppressing them.

## Before you call it done

```sh
cargo fmt --check
cargo test --release
PYAC_REFERENCE_VOLUME=dictionary/reference.pyac cargo test --release
cargo run --release --bin pyac-dictionary -- compile dictionary/reference.pya /tmp/v.pyac
cmp /tmp/v.pyac dictionary/reference.pyac
```

`cargo test` does not build benches or examples, so run these too before pushing:

```sh
cargo build --release --all-targets
cargo bench -p pyac --no-run
```

A rename that misses a bench is invisible to `cargo test` — that is how
`dictionary_index.rs` kept a `key` field after the type renamed it to `case_type`.

If you changed the ISA, the codec, or the volume format and did not run the last
three lines, the work is not verified.
