# The reference dictionary

The vocabulary lives here as two artifacts:

| file | what it is |
|---|---|
| `reference.pya` | the authored source — ordinary Pyash sentences, diffable |
| `reference.pyac` | the compiled v4 volume, what runtimes read |

## Regenerating the volume

```sh
cargo run --release --bin pyac-dictionary -- \
  compile dictionary/reference.pya dictionary/reference.pyac
```

`compile` reads the `.pya` source directly. There is no JavaScript in this path.

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
