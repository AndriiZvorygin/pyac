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

---

# Proposal (Phase 2)

Every `proposed` name below is an **active** sense, verified with
`vocab_suggest`. Confidence is honest: `high` means the gloss states the
operation, `medium` means the sense is close but the word carries other readings,
`none` means nothing active fits and it needs a compound or coining.

## The two wrong-sense names

| current | resolves to (wrong) | proposed | Pyac key | gloss |
|---|---|---|---|---|
| `entry` | `hjac` "submitted for judgment" | `word` (high) | `tlat` | "a unit of language that native speakers can identify" |
| `export` | `sroc` "sell or transfer abroad" | `list` (medium) | `lwat` | "a database containing an ordered array of items" |

`entry` was checked against record, row, item, article, element, member — none
carries the right sense; `word` (`tlat`) is what a dictionary entry is. `export`
is a dump of every key, so it is a listing; `list`/`lwat` fits, though weakly.

## The eight unmoored

| current | proposed | Pyac key | gloss | confidence |
|---|---|---|---|---|
| `inspect` | `examine` | `hzi7n` | "consider in detail and subject to analysis" | high |
| `shard` | `partition` | `cwi2n` | "separate or apportion into sections" | high |
| `benchmark` | `measure` | `pluk` | "evaluate or estimate the nature, quality, ability, extent" | high |
| `component` | `atom` | `kram` | "the smallest component of an element" | high |
| `task` | `duty` | `htup` | "the social force that binds you to courses of action demanded" | high |
| `lexical-form` | `spelling` | `jlin` | "forming words with letters according to accepted usage" | medium |
| `source-gloss` | `translation` | `ryan` | "a written communication in a second language having the same meaning" | medium |
| `definition` | — | — | nothing active states "lowered body / statement" | none |

`atom` is also the vocabulary's own word for the smallest component, and the
source already ships an `atoms` shard — so `component -> atom` is consistent with
the existing split rather than a new coinage.

`definition` is unsolved. `statement` (`fyak`, "a message that is stated or
declared") is the closest active sense, but the field holds already-lowered
codelets, not a statement about them. Do not rename it on a guess.

## Found while checking

- **`surface` is a third wrong-sense CLI verb.** It resolves to `hpum`, "come to
  the surface" — the physical sense. Its job is a language's surface form.
- `tlat` = word, `kwon` = dictionary ("a reference book containing an alphabetical
  list of words"). So the records' own subject `tlat kwon` means **word
  dictionary** — the volume, not an entry. It does not name a record.

## Blast radius, per your "free renames only"

| free now (CLI verbs, identifiers, prose) | deferred (encoded in the volume) |
|---|---|
| `entry`→`word`, `export`→`list`, `inspect`→`examine`, `shard`→`partition`, `benchmark`→`measure`, `surface`→? | `lexical-form`→`spelling`, `source-gloss`→`translation`, `component`→`atom`, `task`→`duty`, `definition`→? |

The right-hand column changes bytes. Renaming the Rust identifiers for those
without the re-publish would leave identifier and format disagreeing, which is
the split-brain this audit exists to remove — so they move together, later.

---

# Revision: search glosses, prefer compounds

The first proposal was atom-stuffing and is withdrawn. With 54 million two-word
compounds available by recombination, the right move is to find the compound whose
**gloss states the concept**, not the atom that is merely nearby.

`vocab_suggest` searches surfaces. What was missing is a search over **glosses**
across all 64,598 entries — `read_volume` already gives every entry with its
source gloss, so a 15-line filter over the decoded model answers "which entry
means X". That search produced every row below; each gloss is quoted verbatim
from the volume, so the link is checkable rather than asserted.

| current | proposed | Pyac key | gloss from the volume |
|---|---|---|---|
| `entry` | entry | `nyit lwat` | an item inserted in a written record |
| `export` | export | `hfat sroc` | transfer (electronic data) out of a database or document in a format that can be used by other programs |
| `inspect` | examine-closely | `zwum hzi7n` | examine closely |
| `shard` | apportion | `syap plon` | the act of apportioning or distributing something |
| `benchmark` | criterion | `rwun` | a basis for comparison; a reference point against which other things can be evaluated |
| `component` | separate-part | `drec pfun` | a separate part of a whole |
| `task` | special-assignment | `htup tra2k` | a special assignment that is given to a person or group |
| `lexical-form` | written-word-form | `hfam tlat` | the written form of a word |
| `source-gloss` | word-meaning | `kfen pcat` | the meaning of a word or expression |
| `definition` | exact-equivalent-definition | `dlut nyif` | a definition that gives an exact equivalent of the term defined |
| `surface` | — | — | only physical-surface senses exist; nothing states a surface *form* |

`benchmark` is `criterion`, not `measure`: `rwun` is already the atom behind
`program/verbs/criterion.mjs`, and its gloss ("a basis for comparison") is the
benchmark sense. `rwun hbis` ("an imaginary line or standard by which things are
measured or compared") is the narrower compound if the verb should be distinct
from `criterion` itself.

## What this says about the search

Substring-AND over glosses was enough for 10 of 11. A real command should do
better than that: tokenized, ranked, and able to match a paraphrase rather than a
literal phrase. That is the retrieval gap again, now from the naming side — and
it is the thing to build before the next tranche of names.

`surface` remains the one with no applicable meaning. It is a third wrong-sense
verb (`hpum`, "come to the surface"), and the concept it needs — "a word's
surface form" — is not stated by any entry.

---

# Applied

**Renamed, free (CLI verbs only):**

| was | now | Pyac key |
|---|---|---|
| `inspect` | `examine` | `hzi7n` |
| `shard` | `apportion` | `syap plon` |
| `benchmark` | `criterion` | `rwun` |

**Citations added at first use** — `compile`/`hk6n`, `verify`/`pfen`,
`parse`/`flis`, `encode`/`kfon`, `decode`/`hkom tyac`, `read`/`tyac`,
`examine`/`hzi7n`, `apportion`/`syap plon`, `criterion`/`rwun`,
`export`/`hfat sroc`, `entry`/`nyit lwat`.

`entry` and `export` needed no rename: their active senses were there all along
(`nyit lwat` "an item inserted in a written record"; `hfat sroc` "transfer
electronic data out of a database..."), just buried under a wrong one.

**Not applied.** The five encoded record kinds (`component`, `task`,
`lexical-form`, `source-gloss`, `definition`) and `surface`. They change bytes, so
they wait for the re-publish; `surface` has no applicable meaning at all.

**Gated.** `vocabulary_naming::vocabulary_words_cite_their_compound` pins the
twelve citations above, so a lost comment fails a test rather than silently
letting a name drift back to being chosen by eye.
