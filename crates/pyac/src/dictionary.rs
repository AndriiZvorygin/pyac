mod decode;
mod index;
mod model;
mod operation;
mod reader;
mod reader_codec;
mod reader_index;
mod shard;
mod source;
mod volume;
mod volume_records;
mod wire;
mod words;

pub use decode::decode_dictionary;
pub use index::DictionaryIndex;
pub use model::{DictionaryEntry, DictionaryIndexError, LexicalForm, SourceGloss, SourceLink};
pub use operation::{
    ArgumentSlot, OperationParameter, OperationResult, OperationSignature, OperationSignatureKey,
};
pub use reader::{DictionaryFileReader, DictionaryReadError, DictionarySection};
pub use shard::{
    BEYOND_LAST_LAYER, EN_SOURCE_EXT, LAYERS, LayerIndex, MANIFEST_FILE, PYAC_SOURCE_EXT,
    REFERENCE_SOURCE_FILE, ShardError, ShardRecord, ShardSet, assemble_shards, parse_manifest,
    shard_candidates, shard_digest, shard_file_name, split_source, split_source_with_layers,
};
pub use source::{
    ArgumentRecord, DictionarySource, EntryRecord, LexicalFormRecord, ResultRecord,
    SignatureRecord, SourceError, SourceGlossRecord, SourceLinkRecord, VolumeMetadataRecord,
    parse_source,
};
pub use volume::{
    DictionaryMetadata, DictionaryVolume, DictionaryVolumeEntry, DictionaryVolumeError,
    encode_dictionary,
};
pub use wire::{
    DictionaryWireError, Field as DictionaryField, Value as DictionaryValue, encode_text_sentence,
};
