# The reference dictionary

The vocabulary lives here as two artifacts:

| file | what it is |
|---|---|
| `reference.en.pya` | the authored English-form Pyash source — diffable records |
| `reference.pyac` | the compiled v4 volume, what runtimes read |

## Two source forms, two extensions

`.en.pya` is the English-form Pyash (`su name … ob text … be text ya`), which is
what the vocabulary records are written in. Plain `.pya` is reserved for the
glyph Pyac form, which is the language itself. The compiler emits `.en.pya`; the
assembler accepts either on read.

## Layout

```
dictionary/
  shard/                      what the repository ships
    00-manifest.en.pya        profile sentences + one row per shard
    atoms.en.pya              7,840 atoms, 17.9 MB
    compounds-16.en.pya       compounds whose deepest word needs 16 glyphs
    compounds-19.en.pya       …then 19, 24, 28, 32
    dispatch/<verb>.en.pya    ceremony signatures, one file per verb
  reference.en.pya            optional whole-source convenience file (git-ignored)
  reference.pyac              the compiled volume (git-ignored)
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

- **dispatch is one file per entry.** A ceremony signature says what an operation
  accepts. It changes when the dispatch API changes, not when the vocabulary
  does, so keeping it beside the entries would make every vocabulary edit reflow
  the signature text and every signature change reflow a vocabulary shard.
  96 entries carry signatures (1,832 records, 1.16 MB); split per entry the
  median file is 3 KB and an API change touches only what it changes.

  Named for the **verb**, not the entry: a signature's verb *is* its entry's
  spelling (measured: 96 entries, 96 distinct verbs, 1:1), so `dispatch/clac.en.pya`
  is "the dispatch the operation `clac` accepts". Assembly places dispatch last
  and nothing in the vocabulary depends on it.

Measured shard sizes: atoms 17.4 MB, compounds-16 15.9 MB, -19 23.9 MB,
-24 19.9 MB, -28 8.9 MB, -32 14.3 MB, plus 96 dispatch files totalling 1.16 MB.
**Every file is far under git's 50 MB warning.**

## Regenerating the volume

```sh
cargo run --release --bin pyac-dictionary -- compile dictionary/shard dictionary/reference.pyac
```

`compile` takes either a shard directory or a single `.en.pya` file, assembles if
needed, and parses. There is no JavaScript in this path.

`compile` is byte-identical to the published volume either way;
`crates/pyac/tests/dictionary_compile.rs` is the gate.

## Re-splitting after an edit

```sh
PYAC_LAYER_LISTS=~/pyac/pyachso2n \
  cargo run --release --bin pyac-dictionary -- \
  shard dictionary/reference.en.pya dictionary/shard
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
```

To edit the volume, edit its source: `decode` it, change the text, `compile` it
back. The next section is that loop. There is no JSON form of the dictionary and
no command that produces one.

## The source round-trip: `decode`

`decode` is the inverse of `compile` — it turns a volume back into source text:

```sh
# write to a file; only the JSON report reaches the terminal
cargo run --release --bin pyac-dictionary -- decode dictionary/reference.pyac /tmp/back.en.pya
cargo run --release --bin pyac-dictionary -- compile /tmp/back.en.pya /tmp/back.pyac
cmp dictionary/reference.pyac /tmp/back.pyac   # identical

# the glyph Pyac form
cargo run --release --bin pyac-dictionary -- decode dictionary/reference.pyac /tmp/back.pya --glyph
```

Pass an output path: a decode is 100 MB, and piping it to a terminal is how it
ends up in a transcript. With no output path the source goes to stdout, so
`decode … > file` also works.

A key is spelled back through the word codec, text stays a JSON string literal
in the English form (`zi.kyit.` frames in the glyph form), and every record kind
is emitted in the order `parse_source` reads it. `crates/pyac/tests/dictionary_render.rs`
and `crates/pyac/tests/dictionary_pyac.rs` are the gates: both decode the
published volume, recompile it, and require the same bytes.

## Where the entries' evidence comes from

Each entry carries `source-link` records citing the dataset it was derived from,
and `source-gloss` records carrying the definition. See `SOURCES.md` for the
dataset list, licences, and how a citation resolves.

A word may carry several references at once — a WordNet sense, a Wiktionary
term, an authored gloss, a SUMO concept. **None of them is the word's identity.**
The Pyac spelling is. WordNet sense ids renumber between releases, so they are
citations, never keys.
