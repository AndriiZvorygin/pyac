# The reference dictionary

The vocabulary lives here as two artifacts:

| file | what it is |
|---|---|
| `reference.pya` | the authored source — ordinary Pyash sentences, diffable |
| `reference.pyac` | the compiled v4 volume, what runtimes read |

## Layout

```
dictionary/
  shard/                    what the repository ships
    00-manifest.pya         profile sentences + one row per shard
    atoms.pya               7,840 atoms, 17.9 MB
    compounds-16.pya        compounds whose deepest word needs 16 glyphs
    compounds-19.pya        …then 19, 24, 28, 32
  reference.pya             optional whole-source convenience file (git-ignored)
  reference.pyac            the compiled volume (git-ignored)
```

The source is split because it is **101.5 MB** — over git's 100 MB per-file hard
limit. The split follows the vocabulary's own dependency order, not byte
offsets:

- **atoms** form one shard. A compound may only name components that are
  themselves entries, so atoms must come first, and they are only 17.9 MB.
- **compounds** group by glyph layer. The layers in `~/pyac/pyachso2n/`
  (16 → 19 → 24 → 28 → 32) each add glyphs to the last, so a word's layer is the
  lowest one that can spell it. A compound's layer is the maximum over its key
  and components, which means a shard never depends on a later one.

Measured shard sizes: atoms 17.9 MB, compounds-16 16.0 MB, -19 23.9 MB,
-24 20.5 MB, -28 8.9 MB, -32 14.3 MB. **All are under half git's 50 MB warning.**

## Regenerating the volume

```sh
cargo run --release --bin pyac-dictionary -- compile dictionary/shard dictionary/reference.pyac
```

`compile` takes either a shard directory or a single `.pya` file, assembles if
needed, and parses. There is no JavaScript in this path.

`compile` is byte-identical to the published volume either way;
`crates/pyac/tests/dictionary_compile.rs` is the gate.

## Re-splitting after an edit

```sh
PYAC_LAYER_LISTS=~/pyac/pyachso2n \
  cargo run --release --bin pyac-dictionary -- \
  shard dictionary/reference.pya dictionary/shard
```

Without `PYAC_LAYER_LISTS` the split still runs but puts every compound in one
shard — correct, just coarser. The glyph lists live in the `pyash` checkout, so
the path is configured rather than assumed.

Assembly verifies each shard's sha256 and entry count, so a hand-edited shard
fails loudly instead of compiling into a plausible-looking volume. It also
rejects a compound that precedes a component, which is the ordering rule the
split exists to honour.

## Working without the volume

The volume is a build artifact, not source, so a fresh clone does not have it.
That is deliberate: it is 131 MB, and it is reproducible from the source beside
it.

The full-strength dictionary gates — byte-for-byte decode/encode identity, every
canonical key reading back, the published-vocabulary index checks — **skip with
a message** when no volume is present rather than failing. A clean checkout is
still buildable and still green; it just runs fewer checks.

Point them at a volume elsewhere with:

```sh
PYAC_REFERENCE_VOLUME=/path/to/reference.pyac cargo test
```

Resolution order is `PYAC_REFERENCE_VOLUME`, then
`dictionary/reference.pyac` beside the workspace root, then the `pyash` checkout
this crate was extracted from.

## The gates that matter

```sh
# decode then encode must reproduce the bytes exactly
cargo test --release --test dictionary_roundtrip

# every canonical key must read back — the guard for the `na` defect
cargo test --release --test dictionary_volume

# edit the volume without touching the source
cargo run --release --bin pyac-dictionary -- decode  dictionary/reference.pyac > v.json
cargo run --release --bin pyac-dictionary -- build   dictionary/reference.pyac < v.json
```

`decode` emits the same JSON shape `build` consumes, so the edit cycle has no
intermediate format to keep in sync. It is byte-identical on the published
volume.

## Where the entries' evidence comes from

Each entry carries `source-link` records citing the dataset it was derived from,
and `source-gloss` records carrying the definition. See `SOURCES.md` for the
dataset list, licences, and how a citation resolves.

A word may carry several references at once — a WordNet sense, a Wiktionary
term, an authored gloss, a SUMO concept. **None of them is the word's identity.**
The Pyac spelling is. WordNet sense ids renumber between releases, so they are
citations, never keys.
