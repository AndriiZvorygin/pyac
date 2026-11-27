//! The records a source document parses into, before they become a volume.

//!
//! These mirror the volume model but keep the authored shape: a record is what a
//! line said, not what the encoder will write.

/// One entry as the source declares it, before encoding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryRecord {
    pub key: Vec<u16>,
    pub lexical_forms: Vec<LexicalFormRecord>,
    pub source_links: Vec<SourceLinkRecord>,
    pub source_glosses: Vec<SourceGlossRecord>,
    pub component_keys: Vec<Vec<u16>>,
    pub task_keys: Vec<Vec<u16>>,
    pub references: Vec<Vec<u16>>,
    pub operation_signatures: Vec<SignatureRecord>,
    pub definition: Option<Vec<Vec<u16>>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexicalFormRecord {
    pub language: String,
    pub surface: String,
    pub normalized: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLinkRecord {
    pub dataset: String,
    pub release: String,
    pub identifier: String,
    pub relation: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGlossRecord {
    pub language: String,
    pub dataset: String,
    pub release: String,
    pub identifier: String,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureRecord {
    pub verb: Vec<u16>,
    pub mood: Vec<u16>,
    pub arguments: Vec<ArgumentRecord>,
    pub results: Vec<ResultRecord>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentRecord {
    pub grammatical_case: Vec<u16>,
    pub noun_class: Vec<u16>,
    pub tag_label: String,
    pub descriptive_specification: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRecord {
    pub noun_class: Vec<u16>,
    pub tag_label: String,
    pub descriptive_specification: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeMetadataRecord {
    pub kind: String,
    pub title: String,
    pub revision: u32,
    pub languages: Vec<String>,
    pub normalization_policy: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionarySource {
    pub metadata: VolumeMetadataRecord,
    pub entries: Vec<EntryRecord>,
}
