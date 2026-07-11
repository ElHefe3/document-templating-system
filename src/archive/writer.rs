use std::io::Write;

use anyhow::{Context, Result};
use crc32fast::Hasher;

use crate::archive::ZipFile;

#[derive(Debug)]
struct ZipEntry {
    file: ZipFile,
    crc32: u32,
    local_offset: u32,
}

pub(crate) fn encode_stored(files: Vec<ZipFile>) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut central_entries = Vec::new();

    for mut entry in files.into_iter().map(ZipEntry::new) {
        entry.local_offset = checked_u32(output.len(), "zip local offset")?;
        write_local_file_header(&mut output, &entry)?;
        output.extend_from_slice(entry.file.name.as_bytes());
        output.extend_from_slice(&entry.file.body);
        central_entries.push(entry);
    }

    let central_offset = checked_u32(output.len(), "zip central directory offset")?;
    for entry in &central_entries {
        write_central_directory_header(&mut output, entry)?;
        output.extend_from_slice(entry.file.name.as_bytes());
    }
    let central_size = checked_u32(
        output.len() - central_offset as usize,
        "zip central directory size",
    )?;
    write_end_of_central_directory(
        &mut output,
        central_entries.len(),
        central_size,
        central_offset,
    )?;

    Ok(output)
}

impl ZipEntry {
    fn new(file: ZipFile) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&file.body);
        Self {
            file,
            crc32: hasher.finalize(),
            local_offset: 0,
        }
    }
}

fn write_local_file_header(output: &mut Vec<u8>, entry: &ZipEntry) -> Result<()> {
    write_u32(output, 0x0403_4b50)?;
    write_u16(output, 20)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u32(output, entry.crc32)?;
    write_file_sizes(output, entry)?;
    write_u16(output, checked_u16(entry.file.name.len(), "zip file name")?)?;
    write_u16(output, 0)
}

fn write_central_directory_header(output: &mut Vec<u8>, entry: &ZipEntry) -> Result<()> {
    write_u32(output, 0x0201_4b50)?;
    write_u16(output, 20)?;
    write_u16(output, 20)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u32(output, entry.crc32)?;
    write_file_sizes(output, entry)?;
    write_u16(output, checked_u16(entry.file.name.len(), "zip file name")?)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u32(output, 0)?;
    write_u32(output, entry.local_offset)
}

fn write_file_sizes(output: &mut Vec<u8>, entry: &ZipEntry) -> Result<()> {
    write_u32(
        output,
        checked_u32(entry.file.body.len(), "zip compressed size")?,
    )?;
    write_u32(
        output,
        checked_u32(entry.file.body.len(), "zip uncompressed size")?,
    )
}

fn write_end_of_central_directory(
    output: &mut Vec<u8>,
    entry_count: usize,
    central_size: u32,
    central_offset: u32,
) -> Result<()> {
    write_u32(output, 0x0605_4b50)?;
    write_u16(output, 0)?;
    write_u16(output, 0)?;
    write_u16(output, checked_u16(entry_count, "zip entry count")?)?;
    write_u16(output, checked_u16(entry_count, "zip entry count")?)?;
    write_u32(output, central_size)?;
    write_u32(output, central_offset)?;
    write_u16(output, 0)
}

fn write_u16(output: &mut Vec<u8>, value: u16) -> Result<()> {
    output.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_u32(output: &mut Vec<u8>, value: u32) -> Result<()> {
    output.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn checked_u16(value: usize, label: &str) -> Result<u16> {
    u16::try_from(value).with_context(|| format!("{label} is too large"))
}

fn checked_u32(value: usize, label: &str) -> Result<u32> {
    u32::try_from(value).with_context(|| format!("{label} is too large"))
}
