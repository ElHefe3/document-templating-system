use std::{fs, path::Path};

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;
use serde_json::Value;

use crate::{
    model::Workspace,
    web::asset_support::{
        content_type_for, is_allowed_asset, percent_decode, percent_encode, safe_upload_name,
        unique_asset_path, AssetResult,
    },
};

const MAX_UPLOAD_BYTES: usize = 5 * 1024 * 1024;
pub use crate::web::asset_support::WebAssetError;

#[derive(Debug, Clone, Serialize)]
pub struct AssetInfo {
    pub name: String,
    pub path: String,
    pub url: String,
    pub size: u64,
    pub extension: String,
}

#[derive(Debug, Clone)]
pub struct ServedFile {
    pub body: Vec<u8>,
    pub content_type: &'static str,
}

pub fn list_assets(workspace: &Workspace) -> Result<Vec<AssetInfo>> {
    fs::create_dir_all(&workspace.assets_dir)?;
    let mut assets = Vec::new();
    for entry in fs::read_dir(&workspace.assets_dir)? {
        let path = entry?.path();
        if !path.is_file() || !is_allowed_asset(&path) {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        assets.push(asset_info(workspace, &path, name)?);
    }
    assets.sort_by_key(|asset| asset.name.to_lowercase());
    Ok(assets)
}

pub fn save_asset(workspace: &Workspace, payload: &Value) -> AssetResult<AssetInfo> {
    let filename = payload
        .get("filename")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            WebAssetError::BadRequest(
                "asset upload requires filename and contentBase64".to_string(),
            )
        })?;
    let content = payload
        .get("contentBase64")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            WebAssetError::BadRequest(
                "asset upload requires filename and contentBase64".to_string(),
            )
        })?;
    let safe_name = safe_upload_name(filename)?;
    let mut encoded = content;
    if let Some((_, rest)) = content.split_once(',') {
        if content.starts_with("data:") {
            encoded = rest;
        }
    }
    let data = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| WebAssetError::BadRequest("asset content must be base64".to_string()))?;
    if data.len() > MAX_UPLOAD_BYTES {
        return Err(WebAssetError::BadRequest(
            "asset upload is larger than 5 MB".to_string(),
        ));
    }
    fs::create_dir_all(&workspace.assets_dir).map_err(WebAssetError::from)?;
    let target =
        unique_asset_path(&workspace.assets_dir, &safe_name).map_err(WebAssetError::from)?;
    fs::write(&target, data).map_err(WebAssetError::from)?;
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&safe_name)
        .to_string();
    asset_info(workspace, &target, name).map_err(WebAssetError::from)
}

pub fn serve_workspace_file(root: &Path, relative_url_path: &str) -> AssetResult<ServedFile> {
    let relative = percent_decode(relative_url_path)?;
    let target = root
        .join(relative.trim_start_matches('/'))
        .canonicalize()
        .map_err(|_| WebAssetError::NotFound("file not found".to_string()))?;
    let root = root
        .canonicalize()
        .map_err(|_| WebAssetError::NotFound("file not found".to_string()))?;
    if !target.starts_with(&root) || !target.is_file() {
        return Err(WebAssetError::NotFound("file not found".to_string()));
    }
    let body = fs::read(&target).map_err(WebAssetError::from)?;
    let content_type = content_type_for(&target);
    Ok(ServedFile { body, content_type })
}

fn asset_info(workspace: &Workspace, path: &Path, name: String) -> Result<AssetInfo> {
    Ok(AssetInfo {
        path: workspace.asset_reference(&name),
        url: format!("/assets/{}", percent_encode(&name)),
        size: path.metadata()?.len(),
        extension: path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_lowercase(),
        name,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{model::init_workspace, test_support::temp_dir};

    #[test]
    fn saves_assets_with_unique_safe_names() {
        let root = temp_dir("web-assets");
        let workspace = init_workspace(&root, None).unwrap();
        let payload = json!({
            "filename": "test upload.svg",
            "contentBase64": "PHN2Zz48L3N2Zz4="
        });

        let first = save_asset(&workspace, &payload).unwrap();
        let second = save_asset(&workspace, &payload).unwrap();

        assert_eq!(first.name, "test-upload.svg");
        assert_eq!(second.name, "test-upload-2.svg");
        assert_eq!(list_assets(&workspace).unwrap().len(), 2);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn serves_files_inside_root_only() {
        let root = temp_dir("web-file");
        let workspace = init_workspace(&root, None).unwrap();
        fs::create_dir_all(&workspace.renders_dir).unwrap();
        fs::write(workspace.renders_dir.join("document.html"), "<main></main>").unwrap();

        let served = serve_workspace_file(&workspace.renders_dir, "document.html").unwrap();

        assert_eq!(served.content_type, "text/html; charset=utf-8");
        assert_eq!(served.body, b"<main></main>");
        assert!(serve_workspace_file(&workspace.renders_dir, "..%2Fdocument.json").is_err());

        fs::remove_dir_all(root).unwrap();
    }
}
