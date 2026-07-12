use std::{
    io::{Cursor, Read},
    str,
};

use anyhow::{bail, Context, Result};
use crc32fast::Hasher;
use flate2::read::DeflateDecoder;

use crate::archive::ZipFile;

pub(crate) fn decode(bytes: &[u8]) -> Result<Vec<ZipFile>> {
    let eocd = find_eocd(bytes).context("template package is missing ZIP directory")?;
    let entry_count = read_u16_at(bytes, eocd + 10)? as usize;
    let central_offset = read_u32_at(bytes, eocd + 16)? as usize;
    let mut offset = central_offset;
    let mut files = Vec::new();

    for _ in 0..entry_count {
        ensure_signature(bytes, offset, 0x0201_4b50, "central directory")?;
        let method = read_u16_at(bytes, offset + 10)?;
        let crc32 = read_u32_at(bytes, offset + 16)?;
        let compressed_size = read_u32_at(bytes, offset + 20)? as usize;
        let uncompressed_size = read_u32_at(bytes, offset + 24)? as usize;
        let name_len = read_u16_at(bytes, offset + 28)? as usize;
        let extra_len = read_u16_at(bytes, offset + 30)? as usize;
        let comment_len = read_u16_at(bytes, offset + 32)? as usize;
        let local_offset = read_u32_at(bytes, offset + 42)? as usize;
        let name_start = offset + 46;
        let name_end = name_start + name_len;
        let name = str::from_utf8(slice(bytes, name_start, name_len)?)
            .context("template package entry name is not UTF-8")?
            .to_string();

        let body = read_entry_body(
            bytes,
            local_offset,
            compressed_size,
            uncompressed_size,
            method,
        )?;
        verify_entry_crc(&name, &body, crc32)?;
        files.push(ZipFile { name, body });

        offset = name_end + extra_len + comment_len;
    }

    Ok(files)
}

fn read_entry_body(
    bytes: &[u8],
    local_offset: usize,
    compressed_size: usize,
    uncompressed_size: usize,
    method: u16,
) -> Result<Vec<u8>> {
    ensure_signature(bytes, local_offset, 0x0403_4b50, "local file header")?;
    let local_name_len = read_u16_at(bytes, local_offset + 26)? as usize;
    let local_extra_len = read_u16_at(bytes, local_offset + 28)? as usize;
    let body_start = local_offset + 30 + local_name_len + local_extra_len;
    let compressed = slice(bytes, body_start, compressed_size)?;
    let body = match method {
        0 => compressed.to_vec(),
        8 => inflate_entry(compressed, uncompressed_size)?,
        other => bail!("unsupported template package ZIP method: {other}"),
    };
    if body.len() != uncompressed_size {
        bail!("template package entry has invalid uncompressed size");
    }
    Ok(body)
}

fn inflate_entry(compressed: &[u8], uncompressed_size: usize) -> Result<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(Cursor::new(compressed));
    let mut body = Vec::with_capacity(uncompressed_size);
    decoder
        .read_to_end(&mut body)
        .context("failed to inflate template package entry")?;
    Ok(body)
}

fn verify_entry_crc(name: &str, body: &[u8], expected: u32) -> Result<()> {
    let mut hasher = Hasher::new();
    hasher.update(body);
    if hasher.finalize() == expected {
        Ok(())
    } else {
        bail!("template package entry failed CRC check: {name}")
    }
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .rposition(|window| window == [0x50, 0x4b, 0x05, 0x06])
}

fn ensure_signature(bytes: &[u8], offset: usize, expected: u32, label: &str) -> Result<()> {
    let found = read_u32_at(bytes, offset)?;
    if found == expected {
        Ok(())
    } else {
        bail!("invalid template package {label} signature")
    }
}

fn slice(bytes: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    bytes
        .get(offset..offset + len)
        .context("template package is truncated")
}

fn read_u16_at(bytes: &[u8], offset: usize) -> Result<u16> {
    let bytes = slice(bytes, offset, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let bytes = slice(bytes, offset, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
