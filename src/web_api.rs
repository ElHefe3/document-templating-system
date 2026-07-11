use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use serde_json::{json, Map, Value};
use tiny_http::Request;

use crate::{
    app_paths::Paths,
    integrations,
    model::validate_document,
    pdf_renderer, render, template_service, web_assets,
    web_http::{
        bad_request, binary_response, internal_error, json_response, read_json, read_json_limited,
        web_asset_error, HttpError, HttpResult, WebResponse,
    },
    web_state::{lock_paths, WebState},
};

pub(crate) fn health(state: &WebState) -> HttpResult<WebResponse> {
    let paths = lock_paths(state)?;
    Ok(json_response(
        &json!({"ok": true, "workspace": paths.workspace.root}),
        200,
    ))
}

pub(crate) fn templates(state: &WebState) -> HttpResult<WebResponse> {
    let paths = lock_paths(state)?;
    Ok(json_response(
        &json!({"ok": true, "templates": template_service::template_summaries(Some(&paths)).map_err(internal_error)?}),
        200,
    ))
}

pub(crate) fn active_template(state: &WebState) -> HttpResult<WebResponse> {
    let paths = lock_paths(state)?;
    let template = integrations::load_active_template(&paths).map_err(internal_error)?;
    Ok(json_response(
        &json!({"ok": true, "template": template}),
        200,
    ))
}

pub(crate) fn document(state: &WebState, key: &str) -> HttpResult<WebResponse> {
    let paths = lock_paths(state)?;
    let document = paths.workspace.load_document().map_err(internal_error)?;
    let mut payload = Map::new();
    payload.insert("ok".to_string(), Value::Bool(true));
    payload.insert(key.to_string(), document);
    Ok(json_response(&Value::Object(payload), 200))
}

pub(crate) fn assets(state: &WebState) -> HttpResult<WebResponse> {
    let paths = lock_paths(state)?;
    let assets = web_assets::list_assets(&paths.workspace).map_err(internal_error)?;
    Ok(json_response(&json!({"ok": true, "assets": assets}), 200))
}

pub(crate) fn save_document(
    state: &WebState,
    request: &mut Request,
    raw_document_body: bool,
) -> HttpResult<WebResponse> {
    let payload = read_json(request, true)?;
    let document = if raw_document_body {
        payload
    } else {
        payload.get("document").cloned().unwrap_or(payload)
    };
    let paths = lock_paths(state)?;
    save_valid_document(&paths, &document)?;
    Ok(json_response(&json!({"ok": true}), 200))
}

pub(crate) fn select_template(state: &WebState, request: &mut Request) -> HttpResult<WebResponse> {
    let payload = read_json(request, true)?;
    let template_ref = payload
        .get("template")
        .and_then(Value::as_str)
        .ok_or_else(|| HttpError::new(400, "template is required"))?;
    let mut paths = lock_paths(state)?;
    let workspace = integrations::use_template(&paths, template_ref).map_err(bad_request)?;
    paths.workspace = workspace;
    let template = integrations::load_active_template(&paths).map_err(internal_error)?;
    Ok(json_response(
        &json!({"ok": true, "template": template}),
        200,
    ))
}

pub(crate) fn render_html(state: &WebState, request: &mut Request) -> HttpResult<WebResponse> {
    save_optional_document(state, request)?;
    let paths = lock_paths(state)?;
    let template = integrations::load_active_template(&paths).map_err(internal_error)?;
    let rendered =
        render::render_html_with_template(&paths.workspace, &template).map_err(internal_error)?;
    let relative_path = rendered
        .html_path
        .strip_prefix(&paths.workspace.root)
        .unwrap_or(&rendered.html_path);
    let preview_url = format!("/{}", relative_path.to_string_lossy().replace('\\', "/"));
    Ok(json_response(
        &json!({
            "ok": true,
            "path": relative_path,
            "previewUrl": preview_url
        }),
        200,
    ))
}

pub(crate) fn render_pdf(state: &WebState, request: &mut Request) -> HttpResult<WebResponse> {
    save_optional_document(state, request)?;
    let paths = lock_paths(state)?;
    let template = integrations::load_active_template(&paths).map_err(internal_error)?;
    let body = pdf_renderer::render_pdf_bytes_with_template(
        &paths.workspace,
        &paths.project_root,
        &template,
    )
    .map_err(internal_error)?;
    Ok(binary_response(
        body,
        "application/pdf",
        Some(format!(
            "attachment; filename=\"{}\"",
            template.render.pdf_filename
        )),
        200,
    ))
}

pub(crate) fn save_asset(state: &WebState, request: &mut Request) -> HttpResult<WebResponse> {
    let payload = read_json(request, true)?;
    let paths = lock_paths(state)?;
    let asset = web_assets::save_asset(&paths.workspace, &payload).map_err(web_asset_error)?;
    Ok(json_response(&json!({"ok": true, "asset": asset}), 200))
}

pub(crate) fn workspace_file(
    state: &WebState,
    area: WorkspaceFileArea,
    relative_url_path: &str,
) -> HttpResult<WebResponse> {
    let paths = lock_paths(state)?;
    let root = match area {
        WorkspaceFileArea::Renders => &paths.workspace.renders_dir,
        WorkspaceFileArea::Assets => &paths.workspace.assets_dir,
    };
    let file =
        web_assets::serve_workspace_file(root, relative_url_path).map_err(web_asset_error)?;
    Ok(binary_response(file.body, file.content_type, None, 200))
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WorkspaceFileArea {
    Renders,
    Assets,
}

// ---------------------------------------------------------------------------
// Remote template upload / delete
// ---------------------------------------------------------------------------

const MAX_UPLOAD_FILES: usize = 256;
const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;
const MAX_UPLOAD_JSON_BYTES: u64 = 30 * 1024 * 1024;

pub(crate) fn remote_template_upload(
    state: &WebState,
    request: &mut Request,
) -> HttpResult<WebResponse> {
    let payload = read_json_limited(request, true, MAX_UPLOAD_JSON_BYTES)?;
    let overwrite = payload
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let files = payload
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| HttpError::new(400, "files array is required"))?;

    if files.is_empty() {
        return Err(HttpError::new(400, "at least one file is required"));
    }
    if files.len() > MAX_UPLOAD_FILES {
        return Err(HttpError::new(
            400,
            format!("too many files: {} (max {MAX_UPLOAD_FILES})", files.len()),
        ));
    }

    let mut decoded: Vec<(String, Vec<u8>)> = Vec::with_capacity(files.len());
    let mut total = 0usize;
    for (index, entry) in files.iter().enumerate() {
        let path = entry
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| HttpError::new(400, format!("files[{index}].path is required")))?;
        let content_base64 = entry
            .get("contentBase64")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                HttpError::new(400, format!("files[{index}].contentBase64 is required"))
            })?;

        if path.is_empty() {
            return Err(HttpError::new(400, "file path must not be empty"));
        }
        if path.contains('\\') {
            return Err(HttpError::new(400, "backslash paths are not allowed"));
        }

        let normalized = normalize_relative_path(path)?;
        if normalized.is_empty() {
            return Err(HttpError::new(400, "file path must not be empty"));
        }

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(content_base64)
            .map_err(|err| {
                HttpError::new(400, format!("invalid base64 in files[{index}]: {err}"))
            })?;
        total += bytes.len();
        if total > MAX_UPLOAD_BYTES {
            return Err(HttpError::new(
                400,
                format!("total upload too large: {total} bytes (max {MAX_UPLOAD_BYTES})"),
            ));
        }
        decoded.push((normalized, bytes));
    }

    let paths = lock_paths(state)?;
    let temp_dir = temp_upload_dir();
    fs::create_dir_all(&temp_dir)
        .map_err(|err| internal_error(format!("failed to create temp dir: {err}")))?;

    let result = (|| -> Result<Value, HttpError> {
        if decoded.len() == 1 {
            let (rel, bytes) = &decoded[0];
            let file_path = temp_dir.join(rel);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    internal_error(format!("failed to create temp subdir: {err}"))
                })?;
            }
            fs::write(&file_path, bytes)
                .map_err(|err| internal_error(format!("failed to write temp file: {err}")))?;

            // Check for conflict
            let preview = integrations::remote_template_upload_preview(&paths, &file_path)
                .map_err(|err| HttpError::new(400, err.to_string()))?;
            if !overwrite && preview.exists {
                let _ = fs::remove_dir_all(&temp_dir);
                return Ok(json!({
                    "ok": false,
                    "conflict": {"id": preview.id, "key": preview.key}
                }));
            }

            let write_result = integrations::upload_remote_template(&paths, &file_path, overwrite)
                .map_err(|err| HttpError::new(400, err.to_string()))?;
            Ok(json!({
                "ok": true,
                "template": {"id": write_result.id, "key": write_result.key, "url": write_result.url}
            }))
        } else {
            for (rel, bytes) in &decoded {
                let file_path = temp_dir.join(rel);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).map_err(|err| {
                        internal_error(format!("failed to create temp subdir: {err}"))
                    })?;
                }
                fs::write(&file_path, bytes)
                    .map_err(|err| internal_error(format!("failed to write temp file: {err}")))?;
            }

            let preview = integrations::remote_template_upload_preview(&paths, &temp_dir)
                .map_err(|err| HttpError::new(400, err.to_string()))?;
            if !overwrite && preview.exists {
                let _ = fs::remove_dir_all(&temp_dir);
                return Ok(json!({
                    "ok": false,
                    "conflict": {"id": preview.id, "key": preview.key}
                }));
            }

            let write_result = integrations::upload_remote_template(&paths, &temp_dir, overwrite)
                .map_err(|err| HttpError::new(400, err.to_string()))?;
            Ok(json!({
                "ok": true,
                "template": {"id": write_result.id, "key": write_result.key, "url": write_result.url}
            }))
        }
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    let value = result?;
    let status = if value.get("ok").and_then(Value::as_bool) == Some(false) {
        409
    } else {
        200
    };
    Ok(json_response(&value, status))
}

pub(crate) fn remote_template_delete(
    state: &WebState,
    request: &mut Request,
) -> HttpResult<WebResponse> {
    let payload = read_json(request, true)?;
    let template_id = payload
        .get("template")
        .and_then(Value::as_str)
        .ok_or_else(|| HttpError::new(400, "template is required"))?;
    let paths = lock_paths(state)?;
    let message = integrations::delete_remote_template(&paths, template_id)
        .map_err(|err| HttpError::new(400, err.to_string()))?;
    Ok(json_response(&json!({"ok": true, "message": message}), 200))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn normalize_relative_path(path: &str) -> Result<String, HttpError> {
    let p = Path::new(path);
    let mut parts = Vec::new();
    for component in p.components() {
        match component {
            Component::Normal(part) => {
                parts.push(part.to_string_lossy().to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(HttpError::new(
                    400,
                    "parent directory traversal is not allowed",
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(HttpError::new(400, "absolute paths are not allowed"));
            }
        }
    }
    Ok(parts.join("/"))
}

fn temp_upload_dir() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string());
    std::env::temp_dir().join(format!("document-templating-system-upload-{suffix}"))
}

fn save_optional_document(state: &WebState, request: &mut Request) -> HttpResult<()> {
    let payload = read_json(request, false)?;
    let Some(payload) = payload.as_object() else {
        return Ok(());
    };
    let document = payload.get("document").cloned();
    let Some(document) = document else {
        return Ok(());
    };
    let paths = lock_paths(state)?;
    save_valid_document(&paths, &document)
}

fn save_valid_document(paths: &Paths, document: &Value) -> HttpResult<()> {
    let template = integrations::load_active_template(paths).map_err(internal_error)?;
    validate_document(&template, document).map_err(bad_request)?;
    paths
        .workspace
        .save_document(document)
        .map_err(internal_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::AppConfig, document_model::set_path, model::init_workspace, test_support::temp_dir,
    };
    use std::fs;

    #[test]
    fn save_valid_document_rejects_invalid_document() {
        let root = temp_dir("web-invalid-document");
        let workspace = init_workspace(&root, None).unwrap();
        let mut document = workspace.load_document().unwrap();
        set_path(&mut document, "profile.full_name", json!(["not a string"])).unwrap();
        let paths = Paths {
            project_root: root.clone(),
            workspace,
            app_config: AppConfig::default(),
        };

        let err = save_valid_document(&paths, &document).unwrap_err();

        assert_eq!(err.status.0, 400);
        assert!(err.message.contains("profile.full_name must be a string"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn normalize_rejects_absolute_path() {
        let err = normalize_relative_path("/etc/passwd").unwrap_err();
        assert_eq!(err.status.0, 400);
        assert!(err.message.contains("absolute paths are not allowed"));
    }

    #[test]
    fn normalize_rejects_parent_traversal() {
        let err = normalize_relative_path("../secret").unwrap_err();
        assert_eq!(err.status.0, 400);
        assert!(err.message.contains("parent directory traversal"));
    }

    #[test]
    fn normalize_accepts_simple_relative_path() {
        let result = normalize_relative_path("template.json").unwrap();
        assert_eq!(result, "template.json");
    }

    #[test]
    fn normalize_accepts_nested_relative_path() {
        let result = normalize_relative_path("subdir/file.html").unwrap();
        assert_eq!(result, "subdir/file.html");
    }

    #[test]
    fn normalize_strips_current_dir() {
        let result = normalize_relative_path("./template.json").unwrap();
        assert_eq!(result, "template.json");
    }
}
