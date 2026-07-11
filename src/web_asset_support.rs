use std::path::{Path, PathBuf};

use anyhow::Result;

const ASSET_EXTENSIONS: &[&str] = &[".png", ".jpg", ".jpeg", ".webp", ".svg"];

#[derive(Debug)]
pub enum WebAssetError {
    BadRequest(String),
    NotFound(String),
    Internal(anyhow::Error),
}

impl std::fmt::Display for WebAssetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(message) | Self::NotFound(message) => formatter.write_str(message),
            Self::Internal(err) => write!(formatter, "{err}"),
        }
    }
}

impl std::error::Error for WebAssetError {}

pub(crate) type AssetResult<T> = std::result::Result<T, WebAssetError>;

pub(crate) fn safe_upload_name(filename: &str) -> AssetResult<String> {
    let path = Path::new(filename);
    if filename.trim().is_empty()
        || path.file_name().and_then(|value| value.to_str()) != Some(filename)
        || filename.contains("..")
    {
        return Err(WebAssetError::BadRequest(
            "filename must not contain path segments".to_string(),
        ));
    }
    if !is_allowed_asset(path) {
        return Err(WebAssetError::BadRequest(format!(
            "unsupported asset type; expected {}",
            ASSET_EXTENSIONS.join(", ")
        )));
    }
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    let safe_stem = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches(['.', '-'])
        .to_string();
    if safe_stem.is_empty() {
        return Err(WebAssetError::BadRequest(
            "filename must contain a safe name".to_string(),
        ));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    Ok(format!("{safe_stem}.{}", extension.to_lowercase()))
}

pub(crate) fn unique_asset_path(root: &Path, filename: &str) -> Result<PathBuf> {
    let target = root.join(filename);
    if !target.exists() {
        return Ok(target);
    }
    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    let suffix = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    for index in 2.. {
        let candidate = root.join(format!("{stem}-{index}.{suffix}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    unreachable!()
}

pub(crate) fn is_allowed_asset(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_lowercase()))
        .unwrap_or_default();
    ASSET_EXTENSIONS.contains(&extension.as_str())
}

pub(crate) fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub(crate) fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

pub(crate) fn percent_decode(value: &str) -> AssetResult<String> {
    let mut bytes = Vec::new();
    let mut chars = value.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let high = chars
                .next()
                .ok_or_else(|| WebAssetError::NotFound("file not found".to_string()))?;
            let low = chars
                .next()
                .ok_or_else(|| WebAssetError::NotFound("file not found".to_string()))?;
            let hex = [high, low];
            let text = std::str::from_utf8(&hex)
                .map_err(|_| WebAssetError::NotFound("file not found".to_string()))?;
            let decoded = u8::from_str_radix(text, 16)
                .map_err(|_| WebAssetError::NotFound("file not found".to_string()))?;
            bytes.push(decoded);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).map_err(|_| WebAssetError::NotFound("file not found".to_string()))
}

impl From<std::io::Error> for WebAssetError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.into())
    }
}

impl From<anyhow::Error> for WebAssetError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn percent_round_trip() {
        let encoded = percent_encode("test upload.svg");
        assert_eq!(encoded, "test%20upload.svg");
        assert_eq!(percent_decode(&encoded).unwrap(), "test upload.svg");
    }

    #[test]
    fn upload_names_are_sanitized() {
        assert_eq!(
            safe_upload_name("test upload.svg").unwrap(),
            "test-upload.svg"
        );
        assert!(safe_upload_name("../bad.png").is_err());
        assert!(safe_upload_name("notes.txt").is_err());
    }

    #[test]
    fn unique_asset_path_uses_numeric_suffixes() {
        let root = temp_dir("assets");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("avatar.png"), b"first").unwrap();

        assert_eq!(
            unique_asset_path(&root, "avatar.png").unwrap(),
            root.join("avatar-2.png")
        );

        fs::remove_dir_all(root).unwrap();
    }
}
