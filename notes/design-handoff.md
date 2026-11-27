# Design handoff — dispatch, dictionary-as-API, heterogeneous operations

Date: 2026-11-27. Baseline: this repo (`~/pyac/pyac`), Rust-only compiled core.
The old `~/pyac/pyash` JS checkout is out of scope and not a conformance target.
Its `documentation/` is kept as historical reference, and
`documentation/suggest/` is where recent work has explored paths toward a
general-intelligence operating system (GI-OS); those notes are proposals
awaiting review, not accepted specifications.

## Rules (from AGENTS.md)
- Rust is normative.
- A word's identity is its spelling; WordNet/SUMO/English surfaces are citations, never keys.
- The dictionary volume is a reproducible build artifact (compile and decode→build byte-identical).

## Dispatch model (agreed)
- A sentence = a verb + a bundle of typed noun phrases in named cases.
- Case is compositional: `source | way | destination × context` — a 12×3 grid,
  not three flat cases. The contexts, in order, are `space`, `interior`,
  `surface`, `under`, `time`, `state`, `person`, `social`, `discourse`,
  `quantity`, `limit`, `sequence`; each `(context, axis)` cell is its own
  morpheme (`time.way = during`, `quantity.way = by`, `interior.way = in`; the
  source form selects `way` with `via`). The canonical contract is
  `~/pyac/pyash/program/library/compositionalCases.mjs`, documented in
  `~/pyac/pyash/documentation/compositional-cases.md`.
  - Eleven identities are intentionally unassigned: the `quantity.way` and
    `quantity.destination` cells, all three `limit` cells, all three `sequence`
    cells, and the `quantity`/`limit`/`sequence` context identities. Composition
    typing that crosses those cells is blocked until they are allocated. This is
    a dependency, not a detail.
- Case axis encodes direction: source = input, way = context, destination = output.
- Noun class = the concrete value class: number, name, text, vector, boolean,
  filename, mind, refinery, map, series, … (vector also names its element class).
- Dispatch identity = `verb + mood + sorted[(grammatical_case, noun_class)]`.
- Input vs output is the case axis, not mood.
- A new operation gets a new key when it has a new verb, or the same verb with a
  new **argument** noun class. Results are not in the key (below), so a
  difference only in the output noun class does *not* create a new key — it
  collides. That is the open question below, not a guarantee.

## Dictionary = machine-readable API reference
- Per operation an `OperationSignature` = verb + mood + parameters + results.
  - parameters = case-keyed inputs; each `ArgumentSlot { grammatical_case, noun_class }`, in the key.
  - results = outputs keyed by `noun_class` only (no case); not in the key.
  - each parameter/result also carries `tag_label` + `descriptive_specification`,
    which are documentation and never in the key (tag_label affects sort only).
- Naming is layered, and the layers are not interchangeable:
  - Concept: **noun class**, the dispatch discriminator.
  - Dictionary model (`crates/pyac/src/dictionary/operation.rs`): `noun_class`
    (not "type"/`value_type`), `grammatical_case` (`gvak`), `parameter`
    (`tlas hrut`), `tag_label` (`tyec dlut`), `descriptive_specification`
    (`drut pfif`). Guarded by `crates/pyac/tests/vocabulary_naming.rs`.
  - Packed runtime (`pyac-vliw-core`, `SignatureType`): the field is
    `value_type`, a descriptor that packs the noun class with quote sort, scalar
    width, and scalar/vector form. `docs/27-pyash-vliw-isa.md` §5.1 documents it
    at that layer; it is accurate there and is not a synonym to rename away.
  - JSON wire (`pyac-dictionary.rs`): keys are `value_type` / `name` /
    `description` on purpose — a published contract with the source producer
    (`program/library/dictionary/source.mjs`). The Rust fields already carry the
    canonical names; the JSON keys are the same values under their legacy
    spellings. Do not rename the wire without the producer.
  - `name` is the `name_` noun class (`gi`, 0x225e); never use it as a
    documentation field.

## Design topic in progress: many heterogeneous operations
- An "expert" = a new operation (new verb, or same verb with a new argument
  noun class) = a new key, so there is no dispatch collision among experts.
- An operation = three parts:
  1. Declaration: signature `(verb, cases, noun classes)` + tag_label +
     descriptive_specification, linked to a WordNet sense.
  2. Body: composition (ceremony/programlet over existing operations) first; a
     new primitive only when a composition is insufficient.
  3. Quiz: the behavior contract / fitness gate.
- Bootstrapping: the core stays fixed; grow only the dictionary and bodies;
  purely additive; each step gated by a quiz.
- Evolutionary programming: operates on declaration + body; the dictionary API
  is the map it reads; quizzes keep what passes (guard against teaching to the
  test).
- Composition typing: expert A's result `noun_class` must match expert B's
  expected parameter `noun_class`.

## What this inherits from the GI-OS analysis
`~/pyac/pyash/documentation/suggest/pyash-as-gios-foundation.md` reviews the
system against five gates and finds two unproven: Gate 3 (goal formation from
findings) and Gate 5 (a demonstrated mutation → evaluate → select → keep loop).
This handoff's evolutionary loop and quiz gate sit directly on Gate 5, so:
- A green quiz suite is necessary for safety but is not an improvement signal.
  "Keep what passes" needs a **held-out** evaluation and retained accept/reject
  evidence, or it teaches to the test.
- The dictionary is not the knowledge core. This handoff is the authored API
  (what a verb accepts and returns). `suggest/knowledge-core.md` (T0–T7) is the
  accumulated claim/evidence store — a different artifact that must not be
  conflated with the vocabulary.

## Seed ladder: what is actually reachable
The intended ladder is `scale → add → sum → max → softmax → attention → block`,
an iWSP capability order (`suggest/iwsp-pyash-implementation-plan.md`, Group B).
Measured against the shared VLIW core today:
- Present: B1 scalar product, B5/B6 softmax (including the numerically stable
  form), B7 argmax.
- Absent: B2 matrix-matrix product (`QKᵀ`, `AV`), B4 ReLU, B9 squared distance,
  B10 layer-norm, B11 residual add, B12 transcendentals (exp/log/sqrt), B15 log
  base 10, B17 cumulative sum, and B18–B20.
Consequence: the ladder is reachable through `softmax`, but **attention is not a
reachable rung** until B2 exists, and `block` needs B11 + B20. A rung is a rung
only when its kernel exists in the core; the ladder must show those gaps rather
than imply the whole order is climbable.

## Open decisions
- Birth contract for an operation: required fields, where the sense link comes
  from, how a quiz binds.
- Seed ladder: the first few operations so each unlocks the next, ordered
  against the kernel gaps above rather than the paper's order.
- Evolutionary loop + gate: proposal generation, acceptance, anti-overfit
  (held-out split and retained evidence, per Gate 5).
- Results keyed by noun_class only — enough, or add the destination case for
  symmetry? As written, two operations that differ only in output collide.
- Unassigned compositional cells: allocate the eleven identities before
  composition typing crosses `quantity`, `limit`, or `sequence`.

## Where things live (this repo)
- Operation/vocabulary model: `crates/pyac/src/dictionary/` (`operation.rs`, `decode.rs`, `volume*.rs`).
- Specs: `docs/27-pyash-vliw-isa.md` (§5.1 dispatch), `docs/28-pyash-vliw-targets.md`,
  `docs/pyac-vliw-dictionary.md`, `docs/pyash-vliw-decode-and-dispatch.md`.
- Compositional cases (historical checkout):
  `~/pyac/pyash/documentation/compositional-cases.md`, contract in
  `~/pyac/pyash/program/library/compositionalCases.mjs`.
- Dictionary source/volume: `dictionary/shard`, `dictionary/SOURCES.md`, `dictionary/reference.pyac`.
- Constraints: `pyac-vliw-core` is `#![no_std]`; CPU/GPU parity; volume byte-identity.

## Historical and exploratory material (not in this repo)
Historical documentation: `~/pyac/pyash/documentation/`. GI-OS exploration:
`~/pyac/pyash/documentation/suggest/` — proposals awaiting review (iWSP
plan and terms, cpu-arc reasoning-as-code, sleep-framework, memo,
swarm-telepathy, sparse-reward, knowledge-core). The baseline does not import
them; port a decision into `docs/` only when it needs to bind.
