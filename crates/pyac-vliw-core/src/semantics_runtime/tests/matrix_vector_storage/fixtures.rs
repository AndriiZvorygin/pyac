//! Codelet builders for the matrix and vector storage tests.
//!
//! Each builder writes one sentence's words and boundary slots, so a test names
//! the sentence it wants rather than its encoding.

use super::super::*;

pub(super) const INPUT: u32 = 0x4123;
pub(super) const OUTPUT: u32 = 0x4567;
/// `psin` = weight. The fixture names a section "weights", so it uses the atom
/// that means weight. It previously used `hgip` (0x4890), which means "cabbage"
/// — a label that told the reader nothing and contradicted the test.
pub(super) const WEIGHTS: u32 = 0x6844;
pub(super) const NUMBER_QUOTE: u32 = 0x881d;
pub(super) const DESTINATION: u32 = 0x243e;
pub(super) const LOCATIVE: u32 = 0x615f;
pub(super) const MULTIPLICATIVE: u32 = 0x6357;
pub(super) const INSTRUMENTAL: u32 = 0x287e;
pub(super) const SCALAR_WORD: u32 = 0x0d62;
pub(super) const PRODUCE_WORD: u32 = 0x8944;
pub(super) const DEONTIC: u32 = 0x295e;
pub(super) const SOURCE: u32 = 0x313e;
pub(super) const NAME_TYPE: u32 = 0x225e;
pub(super) const LOOKUP_FIRST: u32 = 0xec4a;
pub(super) const LOOKUP_SECOND: u32 = 0xa948;
pub(super) const INDEX_FIRST: u32 = 0xa92b;
pub(super) const INDEX_SECOND: u32 = 0x28ea;
pub(super) const NUMBER_WIDTH_TYPE: u32 = 0x327e;

pub(super) fn append_group(
    words: &mut std::vec::Vec<u32>,
    tokens: &[u32],
    boundaries: &[usize],
) -> usize {
    let start = words.len() / CODELET_WORDS;
    for (chunk_index, chunk) in tokens.chunks(CODELET_WORDS - 1).enumerate() {
        let final_chunk = (chunk_index + 1) * (CODELET_WORDS - 1) >= tokens.len();
        let mut codelet = [0; CODELET_WORDS];
        codelet[0] = u32::from(final_chunk);
        codelet[1..=chunk.len()].copy_from_slice(chunk);
        let mut slot = 0;
        while slot < CODELET_WORDS - 1 {
            let offset = chunk_index * (CODELET_WORDS - 1) + slot;
            let is_boundary = boundaries.contains(&offset);
            if is_boundary == final_chunk {
                codelet[0] |= 1 << (slot + 1);
            }
            slot += 1;
        }
        words.extend(codelet);
    }
    start
}

pub(super) fn append_matrix_row_dot(words: &mut std::vec::Vec<u32>, row: u16) -> usize {
    append_group(
        words,
        &[
            NAME_TYPE,
            INPUT,
            ACCUSATIVE,
            NAME_TYPE,
            OUTPUT,
            DESTINATION,
            NUMBER_QUOTE,
            u32::from(row),
            LOCATIVE,
            NAME_TYPE,
            WEIGHTS,
            INSTRUMENTAL,
            SCALAR_WORD,
            PRODUCE_WORD,
            DEONTIC,
        ],
        &[2, 5, 8, 11, 14],
    )
}

pub(super) fn append_embedding_lookup(
    words: &mut std::vec::Vec<u32>,
    alias: u32,
    row: u16,
) -> usize {
    append_group(
        words,
        &[
            NAME_TYPE,
            WEIGHTS,
            SOURCE,
            NAME_TYPE,
            alias,
            DESTINATION,
            NUMBER_QUOTE,
            u32::from(row),
            LOCATIVE,
            LOOKUP_FIRST,
            LOOKUP_SECOND,
            DEONTIC,
        ],
        &[2, 5, 8, 11],
    )
}

pub(super) fn append_embedding_lookup_from_index(
    words: &mut std::vec::Vec<u32>,
    alias: u32,
) -> usize {
    append_group(
        words,
        &[
            NAME_TYPE,
            WEIGHTS,
            SOURCE,
            NAME_TYPE,
            alias,
            DESTINATION,
            NAME_TYPE,
            INDEX_FIRST,
            INDEX_SECOND,
            LOCATIVE,
            LOOKUP_FIRST,
            LOOKUP_SECOND,
            DEONTIC,
        ],
        &[2, 5, 9, 12],
    )
}

pub(super) fn append_embedding_lookup_from_index_at(
    words: &mut std::vec::Vec<u32>,
    alias: u32,
    position: u16,
) -> usize {
    append_group(
        words,
        &[
            NAME_TYPE,
            WEIGHTS,
            SOURCE,
            NAME_TYPE,
            alias,
            DESTINATION,
            NAME_TYPE,
            INDEX_FIRST,
            INDEX_SECOND,
            LOCATIVE,
            NUMBER_QUOTE,
            u32::from(position),
            MULTIPLICATIVE,
            LOOKUP_FIRST,
            LOOKUP_SECOND,
            DEONTIC,
        ],
        &[2, 5, 9, 12, 15],
    )
}

pub(super) fn append_named_dot(words: &mut std::vec::Vec<u32>, alias: u32) -> usize {
    append_group(
        words,
        &[
            NAME_TYPE,
            INPUT,
            ACCUSATIVE,
            NAME_TYPE,
            alias,
            INSTRUMENTAL,
            SCALAR_WORD,
            PRODUCE_WORD,
            DEONTIC,
        ],
        &[2, 5, 8],
    )
}

pub(super) struct MatrixStorage {
    pub(super) descriptors: std::vec::Vec<u32>,
    pub(super) names: std::vec::Vec<u32>,
    pub(super) payload: std::vec::Vec<u32>,
}

impl MatrixStorage {
    pub(super) fn view(&self) -> StorageView<'_> {
        StorageView::new(&self.descriptors, &self.names, &self.payload)
    }
}

pub(super) fn storage_for_f32(
    input: &[f32],
    matrix: &[f32],
    rows: u32,
    columns: u32,
) -> MatrixStorage {
    let input_bytes = (input.len() * 4) as u32;
    let matrix_bytes = (matrix.len() * 4) as u32;
    let descriptors = std::vec![
        2,
        0,
        1,
        0,
        input_bytes,
        4,
        input.len() as u32,
        0xaa6c,
        1,
        columns,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        1,
        1,
        input_bytes,
        matrix_bytes,
        4,
        matrix.len() as u32,
        0xaa6c,
        2,
        rows,
        columns,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let names = std::vec![INPUT, WEIGHTS];
    let payload = input
        .iter()
        .chain(matrix)
        .map(|value| value.to_bits())
        .collect::<std::vec::Vec<_>>();
    MatrixStorage {
        descriptors,
        names,
        payload,
    }
}

pub(super) fn storage_for_f64(
    input: &[f64],
    matrix: &[f64],
    rows: u32,
    columns: u32,
) -> MatrixStorage {
    let input_bytes = (input.len() * 8) as u32;
    let matrix_bytes = (matrix.len() * 8) as u32;
    let descriptors = std::vec![
        2,
        0,
        1,
        0,
        input_bytes,
        8,
        input.len() as u32,
        0xaa6c,
        1,
        columns,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        1,
        1,
        input_bytes,
        matrix_bytes,
        8,
        matrix.len() as u32,
        0xaa6c,
        2,
        rows,
        columns,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let names = std::vec![INPUT, WEIGHTS];
    let payload = input
        .iter()
        .chain(matrix)
        .flat_map(|value| {
            let bits = value.to_bits();
            [bits as u32, (bits >> 32) as u32]
        })
        .collect();
    MatrixStorage {
        descriptors,
        names,
        payload,
    }
}

pub(super) fn storage_with_index(
    input: &[f32],
    matrix: &[f32],
    rows: u32,
    columns: u32,
    index: u16,
) -> MatrixStorage {
    let mut storage = storage_for_f32(input, matrix, rows, columns);
    let index_offset = ((input.len() + matrix.len()) * 4) as u32;
    storage.descriptors[0] = 3;
    storage.descriptors.extend([
        2,
        2,
        index_offset,
        2,
        2,
        1,
        NUMBER_WIDTH_TYPE,
        1,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);
    storage.names.extend([INDEX_FIRST, INDEX_SECOND]);
    storage.payload.push(u32::from(index));
    storage
}

pub(super) fn storage_with_index_sequence(
    input: &[f32],
    matrix: &[f32],
    rows: u32,
    columns: u32,
    indexes: &[u16],
) -> MatrixStorage {
    let mut storage = storage_for_f32(input, matrix, rows, columns);
    let index_offset = ((input.len() + matrix.len()) * 4) as u32;
    storage.descriptors[0] = 3;
    storage.descriptors.extend([
        2,
        2,
        index_offset,
        (indexes.len() * 2) as u32,
        2,
        indexes.len() as u32,
        NUMBER_WIDTH_TYPE,
        1,
        indexes.len() as u32,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);
    storage.names.extend([INDEX_FIRST, INDEX_SECOND]);
    for chunk in indexes.chunks(2) {
        let low = u32::from(chunk[0]);
        let high = chunk.get(1).copied().map(u32::from).unwrap_or(0);
        storage.payload.push(low | (high << 16));
    }
    storage
}
