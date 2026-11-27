# Dictionary sources

Every entry in `reference.pyac` carries `source-link` records naming where its
evidence came from, and `source-gloss` records carrying the definition text. This
file explains what those identifiers mean.

**No datasets are vendored here.** An entry holds a citation — a dataset name, a
release, and an identifier — not a copy of the corpus it came from. The corpus is
regenerable from the upstream release, or readable from a local mount.

## The identity rule, first

A word's identity is its **Pyac spelling**, not any of the references attached to
it. A single entry may carry several at once:

```
srim  (signature)
  wordnet       Open English WordNet 2025     n:06646237-n
  wiktionary    en_all_nopic_2026-08          signature
  sumo          WordNet 3.0 mapping snapshot  Signature
```

WordNet sense ids **renumber between releases.** They are citations, never keys.
Two entries may cite `n:…` identifiers from different WordNet generations — the
volume already does — and nothing breaks, because the spelling is what identifies
the word.

## Datasets

| dataset string | what it is | licence | how it resolves |
|---|---|---|---|
| `wordnet` | Open English WordNet 2025 | Princeton WordNet licence (BSD-style) | sense id `n:06321935-n` |
| `sumo` | SUMO concept links over the WordNet 3.0 mapping snapshot `f0c126842d098a312e390479b38ecfbac018f8ed` | IEEE (SUMO) | concept name, e.g. `ContentBearingObject` |
| `wiktionary` | Wiktionary English, snapshot `en_all_nopic_2026-08` | CC BY-SA 4.0 | article name; the URL is derived (below) |
| `pyash-curated` | definitions authored in this project | same as this repository | the Pyac spelling itself |

### Wiktionary citations resolve to a URL

The snapshot is not stored here. A wiktionary link resolves as:

```
https://zim.zvorygin.ca/content/wiktionary_en_all_nopic_2026-08/<identifier>
```

so the volume needs only the article name and the release. Anything else would
mean shipping a 9 GB corpus to store one-line definitions.

### `pyash-curated` is not a source of truth to cite

It marks a definition this project wrote, because no dictionary had one. It is
honest about being authored, not presented as external evidence. It covers the
grammar vocabulary — the noun classes and the typological terms that neither
WordNet nor Wiktionary defines.

## Why no GOLD

GOLD (General Ontology for Linguistic Description) was evaluated and rejected as
a source. It has good coverage of grammatical case and mood, but its term IRIs
**do not resolve** — `purl.org/linguistics/gold/AccusativeCase` returns 404, as
does the ontology site and the `.owl` purl. The only working URL is the raw
370 KB file, which is not a per-term citation.

A citation that cannot be followed is not a citation. Wiktionary covers 117 of
the 162 grammar atoms and is already mounted, so it wins on the same criteria
that made GOLD attractive.

## Regenerating the volume from source

```sh
cargo run --release --bin pyac-dictionary -- \
  compile dictionary/shard dictionary/reference.pyac
```

Byte-identical to the published volume. `cargo test --test dictionary_compile`
is the gate on that claim.
