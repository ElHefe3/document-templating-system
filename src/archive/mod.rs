mod reader;
mod writer;

#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZipFile {
    pub(crate) name: String,
    pub(crate) body: Vec<u8>,
}

pub(crate) fn encode_stored(files: Vec<ZipFile>) -> Result<Vec<u8>> {
    crate::archive::writer::encode_stored(files)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Vec<ZipFile>> {
    crate::archive::reader::decode(bytes)
}

pub(crate) fn zip_name_from_path(root: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(root)?
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}
