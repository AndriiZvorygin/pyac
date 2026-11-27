//! Pack execution: the shared runtime over lowered codelets.
//!
//! A pack is a flat stream of codelets; these gates drive `execute_pack` and the
//! numeric programlet runtime over hand-lowered fixtures. Each fixture is the exact
//! codelet sequence the compiler produces for a small Pyash program, so a change
//! that alters lowering, frame handling, or the supervisor shows up as a wrong
//! value here rather than as a silent difference in a 131 MB artifact.
//!
//! Modules group by what they exercise, and each carries the programlet constants
//! only it uses — the constants are the bulk of the text, so keeping them local is
//! what keeps every file under 16 KB.
//!
//! - `fixtures`   — the few fixtures several modules share
//! - `numeric`    — literal and arithmetic codelets, fixed16 and f32/f64 quotes
//! - `signed`     — signed arithmetic across frames, and error indices
//! - `frames`     — ceremony calls, local frames, case-argument passing
//! - `loops`      — bounded loops, the loop supervisor, nested frames
//! - `branches`   — conditionals, boolean locals, branch selection
//! - `conditions` — source-level conditionals lowered to pack codelets
//! - `vectors`    — vector arithmetic, width rules, error indices

#[path = "pack_execution/branches.rs"]
mod branches;
#[path = "pack_execution/conditions.rs"]
mod conditions;
#[path = "pack_execution/fixtures.rs"]
mod fixtures;
#[path = "pack_execution/frames.rs"]
mod frames;
#[path = "pack_execution/loops.rs"]
mod loops;
#[path = "pack_execution/numeric.rs"]
mod numeric;
#[path = "pack_execution/signed.rs"]
mod signed;
#[path = "pack_execution/vectors.rs"]
mod vectors;
