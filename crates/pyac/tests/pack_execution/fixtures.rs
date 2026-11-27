//! Shared codelet fixtures for the pack-execution gates.

use pyac::WORDS_PER_CODELET;

pub const PLUS: [u16; WORDS_PER_CODELET] = [
    0x0149, 0x881d, 1, 0x245e, 0x881d, 1, 0x243e, 0x8a64, 0x295e, 0, 0, 0, 0, 0, 0, 0,
];

pub fn float_quote(value: f32) -> [u16; 3] {
    let bits = value.to_bits();
    [0xd01d, bits as u16, (bits >> 16) as u16]
}

pub fn float_arithmetic(left: f32, right: f32, operator: u16) -> [u16; WORDS_PER_CODELET] {
    let left = float_quote(left);
    let right = float_quote(right);
    [
        0x0511, left[0], left[1], left[2], 0x245e, right[0], right[1], right[2], 0x243e, operator,
        0x295e, 0, 0, 0, 0, 0,
    ]
}

pub fn run_numeric_programlet(
    codelets: &[[u16; WORDS_PER_CODELET]],
    step_budget: u32,
) -> pyac_vliw_core::CoreResult {
    let words = codelets
        .iter()
        .flat_map(|codelet| codelet.iter().map(|word| u32::from(*word)))
        .collect::<Vec<_>>();
    let mut symbols = [0; pyac_vliw_core::MAX_PROGRAMLET_SYMBOL_WORDS];
    let mut frames = pyac_vliw_core::ProgramletStack::EMPTY;
    pyac_vliw_core::execute_numeric_programlet(
        &words,
        codelets.len(),
        step_budget,
        &mut symbols,
        &mut frames,
    )
}
