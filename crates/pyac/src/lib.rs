#![forbid(unsafe_code)]

pub mod backend;
pub mod catalog;
pub mod dictionary;
pub mod iwsp;
pub mod literal;
mod pack_view;
pub mod semantic;
mod storage;

pub use storage::{PyacStorageSections, StorageSectionError, prepare_storage_sections};

pub use backend::riscv::{
    BackendError, RiscvProgram, compile_and_execute_numeric_arithmetic, compile_numeric_arithmetic,
    compile_numeric_invert, execute,
};
pub use catalog::{CatalogError, PyacCatalog, parse_catalog_pya};
pub use iwsp::{IwspError, IwspEvaluator, IwspScore, MAX_IWSP_WIDTH, SealedIwspEvaluator};
pub use literal::{
    DateInstruction, VectorElements, VectorInstruction, decode_date, decode_quote_header,
    decode_vector, execute_date, execute_date_codelet, execute_vector, execute_vector_codelet,
    format_date, index_word,
};
pub use pack_view::{PyacPackView, decode_pack_view};
mod execution;

pub use execution::pack::execute_pack;
pub use execution::programlet::execute_shared_programlet;
pub(crate) use execution::programlet::programlet_error;
pub use semantic::{
    ConditionalNumberInstruction, GOLDEN_NUMERIC_PLUS_CODELET, NumericArithmeticInstruction,
    NumericInvertInstruction, NumericOperandCase, NumericOperator, NumericPlusInstruction,
    NumericVectorArithmeticInstruction, NumericVectorOperand, SemanticError,
    decode_conditional_number, decode_numeric_arithmetic, decode_numeric_invert,
    decode_numeric_plus, decode_numeric_vector_arithmetic, execute_conditional_number,
    execute_conditional_number_codelets, execute_numeric_arithmetic,
    execute_numeric_arithmetic_codelet, execute_numeric_invert, execute_numeric_invert_codelet,
    execute_numeric_plus, execute_numeric_plus_codelet, execute_numeric_vector_arithmetic,
};

use std::fmt::{Display, Formatter};

pub const MAGIC: &[u8; 8] = b"PYACVLIW";
pub const FORMAT_VERSION: u16 = 1;
pub const WORD_BITS: u16 = 16;
pub const WORDS_PER_CODELET: usize = 16;
pub const HEADER_BYTES: usize = 32;
pub const CODELET_BYTES: usize = WORDS_PER_CODELET * 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyacPack {
    pub version: u16,
    pub word_bits: u16,
    pub words_per_codelet: u16,
    pub codelet_bytes: u32,
    pub codelets: Vec<[u16; WORDS_PER_CODELET]>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyacError {
    TooManyCodelets,
    TruncatedPack,
    InvalidMagic,
    UnsupportedVersion(u16),
    UnsupportedWordLayout {
        word_bits: u16,
        words_per_codelet: u16,
    },
    InvalidHeader {
        header_bytes: u16,
        codelet_bytes: u32,
        reserved: u32,
    },
    LengthMismatch {
        expected: usize,
        declared: u32,
        actual: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericPackExecutionError {
    pub index: usize,
    pub source: SemanticError,
}

impl NumericPackExecutionError {
    pub fn code(&self) -> &'static str {
        self.source.code()
    }
}

impl Display for NumericPackExecutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "numeric codelet {} failed: {}",
            self.index, self.source
        )
    }
}

impl std::error::Error for NumericPackExecutionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyacValue {
    Unsigned(u64),
    Signed(i64),
    Float32(u32),
    Float64(u64),
    Fixed16(u16),
    Boolean(bool),
    DateEpochMs(u64),
    Vector(VectorElements),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackExecutionError {
    pub index: usize,
    pub source: SemanticError,
}

/// A section result; `section_index` is its source ordinal, while vector order
/// from `execute_pack_sections` records completion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyacSectionResult {
    pub section_index: usize,
    pub name_words: Vec<u16>,
    pub value: PyacValue,
}

fn map_independent_sections<T, F>(section_count: usize, worker_count: usize, execute: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync,
{
    if section_count == 0 {
        return Vec::new();
    }
    let worker_count = worker_count.max(1).min(section_count);
    if worker_count == 1 {
        return (0..section_count).map(execute).collect();
    }

    let next_section = std::sync::atomic::AtomicUsize::new(0);
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let sender = sender.clone();
            let execute = &execute;
            let next_section = &next_section;
            scope.spawn(move || {
                loop {
                    let section_index =
                        next_section.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if section_index >= section_count {
                        break;
                    }
                    if sender.send(execute(section_index)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        receiver.into_iter().collect()
    })
}

impl PackExecutionError {
    pub fn code(&self) -> &'static str {
        self.source.code()
    }
}

impl Display for PackExecutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "PyAC codelet {} failed: {}",
            self.index, self.source
        )
    }
}

impl std::error::Error for PackExecutionError {}

impl PyacError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooManyCodelets => "too-many-codelets",
            Self::TruncatedPack => "truncated-pack",
            Self::InvalidMagic => "invalid-magic",
            Self::UnsupportedVersion(_) => "unsupported-version",
            Self::UnsupportedWordLayout { .. } => "unsupported-word-layout",
            Self::InvalidHeader { .. } => "invalid-header",
            Self::LengthMismatch { .. } => "truncated-pack",
        }
    }
}

impl Display for PyacError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyCodelets => {
                formatter.write_str("PyAC codelet count exceeds the 32-bit header field")
            }
            Self::TruncatedPack => {
                formatter.write_str("PyAC pack is shorter than its declared header")
            }
            Self::InvalidMagic => formatter.write_str("PyAC pack has an invalid magic value"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "PyAC pack version {version} is unsupported")
            }
            Self::UnsupportedWordLayout {
                word_bits,
                words_per_codelet,
            } => write!(
                formatter,
                "PyAC word layout is unsupported: {word_bits} bits, {words_per_codelet} words"
            ),
            Self::InvalidHeader {
                header_bytes,
                codelet_bytes,
                reserved,
            } => write!(
                formatter,
                "PyAC header is invalid: {header_bytes} header bytes, {codelet_bytes} codelet bytes, reserved {reserved}"
            ),
            Self::LengthMismatch {
                expected,
                declared,
                actual,
            } => write!(
                formatter,
                "PyAC length mismatch: expected {expected}, declared {declared}, actual {actual}"
            ),
        }
    }
}

impl std::error::Error for PyacError {}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

pub fn encode_pack(codelets: &[[u16; WORDS_PER_CODELET]]) -> Result<Vec<u8>, PyacError> {
    let count = u32::try_from(codelets.len()).map_err(|_| PyacError::TooManyCodelets)?;
    let payload_bytes = codelets
        .len()
        .checked_mul(CODELET_BYTES)
        .ok_or(PyacError::TooManyCodelets)?;
    let total_bytes = HEADER_BYTES
        .checked_add(payload_bytes)
        .ok_or(PyacError::TooManyCodelets)?;
    let total_bytes_u32 = u32::try_from(total_bytes).map_err(|_| PyacError::TooManyCodelets)?;
    let mut bytes = vec![0; total_bytes];

    bytes[..MAGIC.len()].copy_from_slice(MAGIC);
    write_u16(&mut bytes, 8, FORMAT_VERSION);
    write_u16(&mut bytes, 10, WORD_BITS);
    write_u16(&mut bytes, 12, WORDS_PER_CODELET as u16);
    write_u16(&mut bytes, 14, HEADER_BYTES as u16);
    write_u32(&mut bytes, 16, count);
    write_u32(&mut bytes, 20, CODELET_BYTES as u32);
    write_u32(&mut bytes, 24, total_bytes_u32);
    write_u32(&mut bytes, 28, 0);

    for (codelet_index, codelet) in codelets.iter().enumerate() {
        let start = HEADER_BYTES + codelet_index * CODELET_BYTES;
        for (word_index, word) in codelet.iter().enumerate() {
            write_u16(&mut bytes, start + word_index * 2, *word);
        }
    }
    Ok(bytes)
}

pub fn decode_pack(bytes: &[u8]) -> Result<PyacPack, PyacError> {
    let view = decode_pack_view(bytes)?;
    let mut codelets = Vec::with_capacity(view.codelet_count());
    for index in 0..view.codelet_count() {
        codelets.push(view.codelet(index).expect("validated codelet range"));
    }
    Ok(PyacPack {
        version: view.version(),
        word_bits: view.word_bits(),
        words_per_codelet: view.words_per_codelet(),
        codelet_bytes: view.codelet_bytes(),
        codelets,
        bytes: bytes.to_vec(),
    })
}

pub fn execute_numeric_arithmetic_pack(
    pack: &PyacPack,
) -> Result<Vec<u64>, NumericPackExecutionError> {
    pack.codelets
        .iter()
        .enumerate()
        .map(|(index, codelet)| {
            execute_numeric_arithmetic_codelet(codelet)
                .map_err(|source| NumericPackExecutionError { index, source })
        })
        .collect()
}

/// Executes independent named sections across available CPU threads.
/// Results are returned in completion order and retain their source ordinals.
pub fn execute_pack_sections(
    pack: &PyacPack,
) -> Result<Vec<PyacSectionResult>, PackExecutionError> {
    let prepared_storage =
        prepare_storage_sections(&pack.codelets).map_err(|_| PackExecutionError {
            index: 0,
            source: SemanticError::MalformedStorageSection,
        })?;
    let storage = pyac_vliw_core::StorageView::new(
        &prepared_storage.descriptors,
        &prepared_storage.names,
        &prepared_storage.payload,
    );
    let codelet_count = prepared_storage.executable_codelet_count;
    let words = pack.codelets[..codelet_count].as_flattened();
    execute_pack_sections_with_storage(&words, codelet_count, storage)
}

fn execute_pack_sections_with_storage(
    words: &[impl pyac_vliw_core::VliwWord + Sync],
    codelet_count: usize,
    storage: pyac_vliw_core::StorageView<'_>,
) -> Result<Vec<PyacSectionResult>, PackExecutionError> {
    if !pyac_vliw_core::has_programlet_section_header(words, codelet_count) {
        return Err(PackExecutionError {
            index: 0,
            source: SemanticError::UnsupportedProgramlet,
        });
    }
    let mut sections =
        [pyac_vliw_core::ProgramletSection::EMPTY; pyac_vliw_core::MAX_PROGRAMLET_SECTIONS];
    let section_count =
        pyac_vliw_core::scan_numeric_programlet_sections(words, codelet_count, &mut sections)
            .map_err(|error| PackExecutionError {
                index: 0,
                source: programlet_error(error),
            })?;

    let worker_count = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    map_independent_sections(section_count, worker_count, |section_index| {
        let section = sections[section_index];
        let start = section.codelet_start as usize;
        let value = execute_shared_programlet(
            words,
            start,
            section.codelet_count as usize,
            start,
            &storage,
        );
        value.map(|value| PyacSectionResult {
            section_index,
            name_words: section.name_words[..section.name_length as usize]
                .iter()
                .map(|word| *word as u16)
                .collect(),
            value,
        })
    })
    .into_iter()
    .collect()
}

#[cfg(test)]
#[path = "execution/tests.rs"]
mod tests;
