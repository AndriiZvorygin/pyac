# GUARD and this repository: what transfers, what does not

Status: reading note, not a specification. It records what arXiv 2611.23476
("GUARD: Stabilizing Recursive Self-Improvement with Regret Guidance", Zhang et
al., Google Cloud, Nov 2026) proposes, and which parts bear on the open
decisions in `notes/design-handoff.md`.

## What the paper is

An autonomous agent is a frozen model plus an editable **harness**: prompts,
skills, tool rules, memory. GUARD evolves that harness in a loop with no external
grader and no weight updates. Its four stages:

1. **Checklist.** At Turn 0, before the agent acts, compile the task prompt and
   tool schema into a fixed list of atomic binary criteria. Grade a finished
   trace against them in two layers: deterministic sandbox checks, then semantic
   checks.
2. **Analyze.** Score each criterion's *obedience* under the incumbent and the
   previous harness, then rank by **directional regret**:
   `Priority(c) = Δhurt(c) + Δgap(c)` — how much the last edit *newly broke*,
   plus what is *still unsolved*. Criteria with priority zero are **pinned** so a
   later edit cannot overwrite a mastered rule.
3. **Propose.** Sample M candidate harnesses in parallel from a shared seed
   prompt, score them on the same task batch (group-relative, so batch difficulty
   cancels), then contrast the best and worst candidate to rewrite the proposal
   prompts for the next round. No critic, no gradient.
4. **Gate (Soft-HURT).** Promote a candidate only when its net gain exceeds the
   hurt it causes on tasks that regressed. Rejected edits go to a ledger so the
   same failure is not re-proposed, and an `[iff <condition>]` repair narrows a
   rule that caused isolated drops instead of deleting it.

The load-bearing claim, and the one worth taking seriously: **a scalar score
conflates "newly broken" with "never solved", and that conflation is what makes
self-editing oscillate or forget.** Their measured failure modes are local
overfitting, catastrophic forgetting, and memory pollution — all three are
differences between those two signals, not a lack of signal.

## Where the repository already stands

This repo is not a harness evolver, so most of GUARD is out of scope. But the two
gaps the handoff lists are exactly the two GUARD addresses.

### The quiz gate is currently a regression gate, not a fitness function

`notes/design-handoff.md` says each operation carries a *quiz* and that
"quizzes keep what passes", and flags Gate 5 of
`pyash-as-gios-foundation.md`: *"evaluation machinery present, improvement loop
unproven."* The paper names why that is not enough: a green regression suite
cannot tell a **newly broken** behaviour from an **old gap**, and cannot rank one
candidate over another, so it cannot drive selection. Its Theorem 2 also shows
the cost of the current shape: an invariant surfaced only by a coarse end-to-end
failure with probability ≤ ε takes Ω(log J / ε) rounds to discover, whereas a
dense pre-declared checklist makes it explicit every round. **The paper's fix is
to decompose the gate into named, atomic, binary criteria and grade them
individually** — which is what a per-operation quiz already is, if it is written
as a list of criteria rather than one pass/fail.

### The dictionary is already a harness-like store, but it has no pinning

`OperationSignature` is a declaration; the body is a composition; the quiz is the
contract. What is missing is any record of **which criteria are mastered and must
not regress**. That is GUARD's pinned set: priority-zero criteria that a new
candidate cannot overwrite. Here it would be a field on the entry or a separate
record kind, and it is algorithmic — the criteria are the ones the incumbent
passes and the previous one did not.

## What maps, concretely

| GUARD concept | What it would be here |
|---|---|
| task checklist `C(q)` | the criteria of one operation's quiz, each atomic and binary |
| obeyed / priority ranking | compare incumbent body vs previous body on the same criteria |
| pinned mastered rules | a *mastered* set on the entry, not a free-text note |
| `[iff <condition>]` precondition | the `noun_class` / case guard on an operation's arguments — already a precondition in the type, so a rule that over-triggers can be narrowed structurally instead of deleted |
| group-relative selection | the same quiz run over M candidate bodies; no critic, no weights |
| Soft-HURT gate | promote a body only if new passes exceed new failures on the incumbent's criteria |
| rejection ledger | a record kind holding rejected bodies and the criteria they broke |

The `[iff]` row is the strongest fit. GUARD's repair step narrows a rule that
caused isolated regressions. Here the narrowing is already the dispatch identity:
an expert is `verb + mood + sorted[(case, noun_class)]`, so a composition that
over-triggers on one noun class can be split into a narrower signature instead of
being thrown away. That is a structural `[iff]`, not a prose one.

## What does not transfer

- **Prompts, tool schemas, MCP.** The paper's harness is prompts and tool rules.
  The core here is a VLIW ISA; there is no prompt to evolve.
- **Their benchmarks** (PAST-Bench, EvoHarnessBench, Drift Gauntlet) depend on
  harnesses this repo does not have.
- **The theoretical results** are about self-review scores on gated task buffers,
  not about byte identity. They do not weaken or strengthen the volume gates.
- **Teacher–student distillation.** Out of scope; the core is not a model.

## What to do, if anything

The paper argues for a specific shape, and the smallest honest step here is one
change, not a framework:

1. **Write the quiz as criteria, not one assertion.** Each quizable operation
   declares a list of atomic binary checks (input accepted, output noun class,
   error case, determinism).
2. **Record the mastered set.** A record kind that names the criteria the current
   body passes, which a new body must not regress.
3. **Gate on directional regret.** Accept a candidate body only when newly-passed
   criteria outnumber newly-broken ones against that set.

Step 3 is the whole contribution of the paper in one sentence, and it is the one
thing the current "keep what passes" rule cannot express. Do not build the
candidate pool or the contrastive prompt evolution until (1)–(2) exist and a gate
on them is green; those are optimisations of a loop that does not exist yet.

## The trap this paper names that applies directly

The reason a byte-identity gate is not a fitness function is the same reason a
regression suite is not one: **it tells you the past still works, never whether
the present is better.** This repo already has unusually good regression gates
(the volume round-trip, the parity shape test). What it does not have, and what
GUARD is about, is a signal that ranks two working states. Keep the distinction:
byte identity is a *safety* gate; a criterion-level regret gate would be a
*selection* gate. They are not substitutes.
