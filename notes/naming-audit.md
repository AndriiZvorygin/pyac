# Naming audit: what the vocabulary says about our names

Phase 1–2 of `~/.commandcode/plans/pyac-naming-alignment.md`. Every verdict below
comes from `node command/vocab_suggest.mjs <word>` in `~/pyac/pyash`, which lists
every WordNet/SUMO sense, marks which are **active** in the reference volume, and
gives the Pyac compound each maps to.

An *active* sense is one the volume already carries. A name with no active sense
is not wrong so much as unmoored: there is no entry it refers to.

## A. Grounded — the sense matches and is active

| name | Pyac key | gloss | note |
|---|---|---|---|
| `compile` | `hk6n` | "translate source code … into machine code that can be executed" | exact; keep |
| `verify` | `pfen` | "confirm the truth of" | also has `xwon` "affirm solemnly" — cite `pfen`, not `xwon` |

These are the only two CLI verbs that land cleanly. Both should cite their key in
a doc comment, which neither does today.

## B. Wrong sense — active, but a different meaning

| name | what it currently resolves to | what we mean |
|---|---|---|
| `entry` | `hjac` — "something submitted for the judgment of others (as in a competition)" | one record in the dictionary |
| `export` | `sroc` — "sell or transfer abroad" | write the volume's keys out |

Both read plausibly in English and point at the wrong concept — the same defect
`operation.rs` documents for `SU`. These are the two worst offenders, because
`vocab_suggest` marks them **active**, so a reader who checks gets a confident
wrong answer.

## C. No active sense — unmoored

| name | what `vocab_suggest` offers instead |
|---|---|
| `inspect` | (nothing active) |
| `shard` | alternatives: fragmentation, oblige |
| `benchmark` | alternatives: cursor, referral, criterion |
| `lexical` | (nothing) |
| `gloss` | alternatives: interpret, varnish, lustrum, shiny, glaze |
| `component` | alternatives: module, complement |
| `definition` | alternatives: denote, connotation, justice |
| `task` | alternatives: affairs, duty, pregnancy |

These eight are domain names — five of them are **record kinds written into the
volume** (`gloss`, `component`, `definition`, `task`, and `lexical` inside
`lexical-form`). The alternatives the tool proposes are visibly wrong, which is
the signal that the concept needs a compound, not a different bare word.

## D. Ambiguous — more than one sense, need to choose

| name | senses |
|---|---|
| `reference` | `hdi7s` "computer address"; `hrec` "citation / acknowledgment" |
| `render` | 11 active; closest are `dra2s` "depicted" and `cwas hsok` "generate" |

`reference` is easy: we mean a pointer to another key, which is the address
sense (`hdi7s`) if anything — but check whether we mean the citation sense.

## E. `render`, specifically

The operation is "decode the volume into its source text". No active sense
covers it; the tool's closest are wrong. Candidates measured:

| word | active sense | Pyac key |
|---|---|---|
| `print` | "put into print" | `trat` |
| `write` | "produce a literary work" | `lyit` |
| `say` | "express in words" | `hsac` |
| `emit` | (nothing active) | — |
| `produce` | "bring forth or yield" | `psas` |

`print` is the closest active sense. Whether it names the operation or the
operation is better left to a compound is your call — this is exactly the
judgement I got wrong before by guessing.

## What the audit says overall

- **2 of 13 CLI verbs** are correctly grounded.
- **2 carry an active but wrong sense** — the most dangerous class.
- **8 domain names have no active sense**, five of which are encoded in the volume.
- The four names already grounded in `operation.rs` (`gvak`, `tlas hrut`,
  `tyec dlut`, `drut pfif`) are the model to follow, not the exception.

## Blast radius of fixing them

| class | names | cost |
|---|---|---|
| free (code/doc only) | `render`, `inspect`, `shard`, `benchmark`, Rust identifiers | rename + commit |
| format-changing (encoded in volume) | `entry`, `lexical-form`, `source-link`, `source-gloss`, `component`, `task`, `reference`, `definition` | recompile, re-publish LFS volume, re-baseline `dictionary_compile` |

The second row is why this must not be bundled with the first: a byte-identity
failure needs one cause.

## Open questions for you

1. `render` → `print`, or a compound? (`trat` is the only active near-fit.)
2. Do tooling verbs (`inspect`, `shard`, `benchmark`) need vocabulary grounding at
   all, or are they developer commands outside the language?
3. For the five encoded record kinds, is a rename in scope now, accepting a
   volume re-publish, or do we only rename the free identifiers first?
