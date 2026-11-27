use super::super::OperationSignatureKey;

#[derive(Debug, Clone, Copy)]
pub(crate) enum DirectIndexLookup<'a> {
    Canonical(&'a [u16]),
    Surface(&'a str, &'a str),
    Source(&'a str, &'a str, &'a str),
    Operation(&'a OperationSignatureKey),
}
#[derive(Debug)]
pub(crate) struct DirectIndexRecord {
    pub(crate) key: DirectIndexKey,
    pub(crate) offsets: Vec<u32>,
    pub(crate) next: u32,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DirectIndexKey {
    Canonical(Vec<u16>),
    Surface(Vec<u16>, String),
    Source(String, String, String),
    Operation(Vec<u16>),
}
impl DirectIndexKey {
    pub(crate) fn with_offsets(self, offsets: Vec<u32>, next: u32) -> DirectIndexRecord {
        DirectIndexRecord {
            key: self,
            offsets,
            next,
        }
    }
}
