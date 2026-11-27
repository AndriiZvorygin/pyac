---
name: pyac-vocabulary
description: Work with the Pyac dictionary — the vocabulary volume, its shards, the glyph (.pya) and English (.en.pya) source forms, and the VLIW word/quote/vector encoding. Use when adding a dictionary entry, changing the source grammar, touching the volume format, rendering a volume back to source, or debugging a word/quote/vector round-trip.
---

# Working with the Pyac vocabulary

> **This is plain Markdown, not a harness plugin.** The frontmatter above is
> optional metadata for tools that read it; nothing here depends on a particular
> agent harness. Read it directly, or point any agent at `skills/README.md`.
> The repository is the source of truth; the skill is a reading path through it.

The vocabulary is a **131 MB volume** (`dictionary/reference.pyac`, on LFS) compiled
from **English-form source shards** (`dictionary/shard/*.en.pya`). Rust is
normative. Nothing here uses JavaScript.

## The three invariants

1. **A word's identity is its spelling.** A WordNet sense id (`n:06321935-n`), a
   SUMO concept, or an English surface is a *citation*, never a key. Sense ids
   renumber between releases; the volume already holds ids from two generations.
   Never build a lookup keyed on a sense id.
2. **The volume is a build artifact.** It is reproducible from the shards and is
   the gate on every change: `compile` must reproduce it byte for byte.
3. **Decode is algorithmic.** Words come from the word codec (`word_code`), record
   kinds from the case sequence, text from quote extents. If your change needs a
   lookup table, it is wrong — that is why the language-name field stays an ISO
   code inside a text quote, not a vocabulary word.

## The two source forms

| extension | form | example |
|---|---|---|
| `.en.pya` | English Pyash | `su name tlat kwon ob text "sa" at text "entry" be text ya` |
| `.pya` | glyph Pyac | `gi sa na zi.kyit.en.kyit.zi swih … hwus li` |

`.en.pya` is what the shards are written in and what `parse_source` reads. Plain
`.pya` is the language itself: the codelet words spelled. Both round-trip the
whole volume byte-identically:

```sh
# English form
cargo run --release --bin pyac-dictionary -- decode dictionary/reference.pyac > /tmp/r.en.pya
cargo run --release --bin pyac-dictionary -- compile /tmp/r.en.pya /tmp/r.pyac && cmp /tmp/r.pyac dictionary/reference.pyac

# glyph form (via the library; `decode_pyac_source`, gated by tests/dictionary_pyac.rs)
```

## The commands you will use

```sh
# regenerate the volume from the shards, then verify
cargo run --release --bin pyac-dictionary -- compile dictionary/shard /tmp/v.pyac
cargo run --release --bin pyac-dictionary -- verify  /tmp/v.pyac

# what the volume says
cargo run --release --bin pyac-dictionary -- surface-entries dictionary/reference.pyac en "noun class"
cargo run --release --bin pyac-dictionary -- entry   dictionary/reference.pyac 0x253e
cargo run --release --bin pyac-dictionary -- export  dictionary/reference.pyac

# the volume back to source, and back again: both byte-identical
cargo run --release --bin pyac-dictionary -- decode dictionary/reference.pyac /tmp/back.en.pya
cargo run --release --bin pyac-dictionary -- compile /tmp/back.en.pya /tmp/back.pyac
```

There is no JSON form of the dictionary: the source *is* the intermediate form.

The dictionary gates **skip** when no volume is present, so a clean clone is
green. Point them at the real volume to run the strong checks:

```sh
PYAC_REFERENCE_VOLUME=dictionary/reference.pyac cargo test --release
```

## The record grammar (glyph form)

A record is a subject, `na`, then `value case` pairs, closed by `hwus li`:

```
gi <key> na <value> <case> … <case> hwus li
```

**The record kind is implied by its case sequence** — there is no `"entry"` tag.
The sequences `decode_entry` accepts:

| case sequence | record |
|---|---|
| `swih`, `so`, `ma` | lexical form (language, surface, normalized) |
| `swih`, `so`, `ma`, `yu` | source link (dataset, release, identifier, relation) |
| `swih`, `so`, `ma`, `yu`, `mwah` | source gloss (adds the text) |
| `so` (vector) | component key |
| `mwah` (vector) | task key |
| `ma` (vector) | reference key |
| `yu` (vector) | operation signature (dispatch) |
| `swih ve`, `so`, `ma` | operation argument slot |

Case words: `na` nominative, `ka` accusative, `swih` essive, `so` source, `ma`
destination, `yu` instrumental, `mwah` comitative, `lwoh` locative. Mood `li`.
Verb `hwus`.

## Values

**Text** is a `zi` frame: `zi.kyit.<payload>.kyit.zi`. `kyit` is the text noun
class; `zi` is `quoted_` (`0x229e`). A literal `.kyit.zi` inside the payload is
escaped with a backslash. Text of **15 or more UTF-16 units** spills into *raw
continuation codelets* where all sixteen slots are payload (no summary word).

**Numbers are not `zi` frames.** They are decimal digit words after a width word:
`do` 16-bit, `po` 8-bit, `pu` 32-bit, `tl6h` 64-bit; `tc6h` marks signed, `fyih`
marks float. `dohyiktyut` is **twelve** (decimal), not eight (base 6).

**Vectors** are `ve <count> <lanes…>`: `ve` is uint16, `pu ve` uint32,
`tc6h pu ve` int32, `fyih pu ve` float32. A wide lane is little-endian, low word
first. Two, three, four, eight and sixteen lanes use a fixed-thickness header;
other lengths carry an explicit count.

**A key that spells a grammar word** (`li`, `na`, `ve`, `pu`, …) is word-quoted:
`wo` emits no word and marks the next root as data, so key `na` is
`giwona na` — key, then the closing `na`. Quoting the word quote itself is
`giwowo` (`wo wo`).

## Traps that have already cost time

- **A payload word can look like a vocabulary word.** The codec is total over
  glyph shapes, so `0x0068` spells `hciEm`; inside a text quote it is `h`.
  Never classify a word without its quote extent — that mistake rendered a 131 MB
  volume as 1 GB of nonsense.
- **Boundary polarity.** The writer marks boundaries *positively* in a sentence's
  final codelet and *negatively* in earlier ones (`slot_is_boundary`).
- **`gi` opens a name only at a value position.** `prifgika` contains `gi` and is
  just a word.
- **A key is data.** It never carries a boundary, whatever its class; only the
  closing `na` closes the name.
- **The source writes case-then-value; the volume writes value-then-case.**
- **`ve` runs are decimal.** `10590` is a number, not hex; `0x`-prefixed is hex.
- **`cargo test` does not build benches or examples.** Run
  `cargo build --release --all-targets` and `cargo bench -p pyac --no-run`.

## Adding an entry

Edit a shard under `dictionary/shard/` (atoms stay whole; compounds group by glyph
layer 16 → 19 → 24 → 28 → 32), then:

```sh
cargo run --release --bin pyac-dictionary -- compile dictionary/shard /tmp/v.pyac
cmp /tmp/v.pyac dictionary/reference.pyac   # must differ only by your entry
```

Then regenerate the published volume and re-baseline the LFS artifact — a format
change means the shipped bytes change.

## Where the rules live

| question | file |
|---|---|
| the instruction set, word classes, quote framing | `docs/27-pyash-vliw-isa.md` |
| the volume format, record kinds, indexes | `docs/pyac-vliw-dictionary.md` |
| the pack + catalog pair | `docs/pyac-vliw-database.md` |
| decoding and dispatch | `docs/pyash-vliw-decode-and-dispatch.md` |
| where the vocabulary's evidence comes from | `dictionary/SOURCES.md` |
| the word codec and glyph packing | `crates/pyac-vliw-core/src/semantics_runtime/word_code.rs` |
| the record encoder | `crates/pyac/src/dictionary/wire.rs` |
| the record decoder | `crates/pyac/src/dictionary/decode.rs` |
| the tablet parser (glyph text → codelets) | `crates/pyac-vliw-core/src/semantics_runtime/text_encode.rs` |
