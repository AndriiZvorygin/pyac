mod decode;
mod index;
mod model;
mod operation;
mod reader;
mod reader_codec;
mod reader_index;
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
