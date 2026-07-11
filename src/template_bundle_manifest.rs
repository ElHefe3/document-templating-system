use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    model::{hydrate_template_render_files, validate_template_manifest, TemplateManifest},
    template_bundle_manifest_discovery::render_file_path,
};

pub(crate) fn load_manifest_template_bundle(
    root: &Path,
    manifest_path: &Path,
) -> Result<TemplateManifest> {
    let mut raw: Value = serde_json::from_str(&fs::read_to_string(manifest_path)?)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
    let id = template_id_from_manifest(root, manifest_path, &raw)?;
    let root_object = raw
        .as_object_mut()
        .context("template manifest must be a JSON object")?;
    let missing_id = root_object
        .get("id")
        .and_then(Value::as_str)
        .map(|value| value.trim().is_empty())
        .unwrap_or(true);
    if missing_id {
        root_object.insert("id".to_string(), Value::String(id.clone()));
    }
    inject_render_file(root, &mut raw, "html", &id, &["template.html"])?;
    inject_render_file(root, &mut raw, "css", &id, &["style.css", "template.css"])?;
    let mut template: TemplateManifest = serde_json::from_value(raw)
        .with_context(|| format!("failed to load template bundle {}", root.display()))?;
    if template.id.trim().is_empty() {
        template.id = id;
    }
    hydrate_template_render_files(&mut template, Some(root))?;
    validate_template_manifest(&mut template, manifest_path)?;
    Ok(template)
}

pub(crate) fn canonical_manifest_body(template: &TemplateManifest) -> Result<Vec<u8>> {
    let mut manifest = template.clone();
    if manifest.render.html_file.is_some() {
        manifest.render.html.clear();
    }
    if manifest.render.css_file.is_some() {
        manifest.render.css.clear();
    }
    Ok(serde_json::to_vec_pretty(&manifest)?)
}

fn template_id_from_manifest(root: &Path, manifest_path: &Path, raw: &Value) -> Result<String> {
    if let Some(id) = raw
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(id.to_string());
    }

    let stem = manifest_path
        .file_stem()
        .and_then(|value| value.to_str())
        .context("template id is required")?;
    if matches!(stem, "template" | "manifest") {
        return root
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
            .context("template id is required");
    }
    Ok(stem.to_string())
}

fn inject_render_file(
    root: &Path,
    raw: &mut Value,
    field: &str,
    template_id: &str,
    fallback_names: &[&str],
) -> Result<()> {
    let render = raw
        .as_object_mut()
        .context("template manifest must be a JSON object")?
        .entry("render")
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let render = render
        .as_object_mut()
        .context("template render must be an object")?;
    let file_field = format!("{field}_file");
    if render
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
        || render
            .get(file_field.as_str())
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(());
    }
    if let Some(path) = render_file_path(root, field, template_id, fallback_names)? {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        render.insert(field.to_string(), Value::String(text));
        if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
            render.insert(file_field, Value::String(name.to_string()));
        }
    }
    Ok(())
}
