# Design handoff — dispatch, dictionary-as-API, heterogeneous operations

Date: 2026-11-26. Baseline: this repo (`~/pyac/pyac`), Rust-only compiled core.
The old `~/pyac/pyash` JS checkout is out of scope and not a conformance target.

## Rules (from AGENTS.md)
- Rust is normative.
- A word's identity is its spelling; WordNet/SUMO/English surfaces are citations, never keys.
- The dictionary volume is a reproducible build artifact (compile and decode→build byte-identical).

## Dispatch model (agreed)
- A sentence = a verb + a bundle of typed noun phrases in named cases.
- Case = grammatical role/axis: source = input, way = context, destination = output.
- Noun class = the concrete value class: number, name, text, vector, boolean, filename,
  mind, refinery, map, series, … (vector also names its element class).
- Dispatch identity = `verb + mood + sorted[(grammatical_case, noun_class)]`.
- Input vs output is the case axis, not mood.

## Dictionary = machine-readable API reference
- Per operation an `OperationSignature` = verb + mood + parameters + results.
  - parameters = case-keyed inputs; each `ArgumentSlot { grammatical_case, noun_class }`, in the key.
  - results = outputs keyed by `noun_class` only (no case); not in the key.
  - each parameter/result also carries `tag_label` + `descriptive_specification`, which are
    documentation and never in the key (tag_label affects sort only).
- Canonical vocabulary (`crates/pyac/src/dictionary/operation.rs`):
  - `noun_class` (not "type" / "value_type")
  - `grammatical_case` (`gvak`)
  - `parameter` (`tlas hrut`) — the case-keyed input
  - `tag_label` (`tyec dlut`) — short doc; NOT `name`
  - `descriptive_specification` (`drut pfif`) — long doc; NOT `description`
  - `name` = the `name_` noun class (`gi`, 0x225e); never use it as a documentation field.

## Design topic in progress: many heterogeneous operations
- An "expert" = a new operation (new verb, or same verb with a new noun class) = a new key,
  so there is no dispatch collision. Same-key multiplicity is backends/parity (target-driven),
  not "experts".
- An operation = three parts:
  1. Declaration: signature `(verb, cases, noun classes)` + tag_label + descriptive_specification,
     linked to a WordNet sense.
  2. Body: composition (ceremony/programlet over existing operations) first; a new primitive
     only when a composition is insufficient.
  3. Quiz: the behavior contract / fitness gate.
- Bootstrapping: the core stays fixed; grow only the dictionary and bodies; purely additive;
  each step gated by a quiz.
- Evolutionary programming: operates on declaration + body; the dictionary API is the map it
  reads; quizzes keep what passes (guard against teaching to the test).
- Composition typing: expert A's result `noun_class` must match expert B's expected parameter
  `noun_class`.

## Open decisions
- Birth contract for an operation: required fields, where the sense link comes from, how a quiz binds.
- Seed ladder: the first few operations so each unlocks the next (iwsp-style parentage:
  scale → add → sum → max → softmax → attention → block).
- Evolutionary loop + gate: proposal generation, acceptance, anti-overfit.
- Results keyed by noun_class only — enough, or add the destination case for symmetry?

## Where things live (this repo)
- Operation/vocabulary model: `crates/pyac/src/dictionary/` (`operation.rs`, `decode.rs`, `volume*.rs`).
- Specs: `docs/27-pyash-vliw-isa.md` (§5.1 dispatch), `docs/28-pyash-vliw-targets.md`,
  `docs/pyac-vliw-dictionary.md`, `docs/pyac-vliw-decode-and-dispatch.md`.
- Dictionary source/volume: `dictionary/shard`, `dictionary/SOURCES.md`, `dictionary/reference.pyac`.
- Constraints: `pyac-vliw-core` is `#![no_std]`; CPU/GPU parity; volume byte-identity.

## Not part of this baseline
The old `pyash` repo's suggest notes (memo, gzip, sleep, iwsp §A.12, knowledge-core) are not
here; port deliberately into `docs/` only if a decision needs them.
