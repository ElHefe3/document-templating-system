use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde_json::{Map, Value};

use crate::model::{validate_document, TemplateManifest, Workspace};

#[derive(Debug, Clone)]
pub struct RenderedHtml {
    pub html_path: PathBuf,
    pub backup_path: Option<PathBuf>,
}

pub fn render_html_with_template(
    workspace: &Workspace,
    template: &TemplateManifest,
) -> Result<RenderedHtml> {
    let document = workspace.load_document()?;
    validate_document(template, &document)?;
    let html = render_html_string(template, &document)?;

    fs::create_dir_all(&workspace.renders_dir)?;
    let backup_path = backup_existing_html(workspace)?;
    fs::write(&workspace.html_path, html)
        .with_context(|| format!("failed to write {}", workspace.html_path.display()))?;
    Ok(RenderedHtml {
        html_path: workspace.html_path.clone(),
        backup_path,
    })
}

pub fn render_html_string(template: &TemplateManifest, document: &Value) -> Result<String> {
    let mut registry = Handlebars::new();
    registry.register_escape_fn(handlebars::html_escape);
    registry
        .register_template_string(&template.id, &template.render.html)
        .with_context(|| format!("failed to compile template {}", template.id))?;
    let context = render_context(template, document);
    registry
        .render(&template.id, &context)
        .with_context(|| format!("failed to render template {}", template.id))
}

fn render_context(template: &TemplateManifest, document: &Value) -> Value {
    let mut map = match document {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    map.insert("document".to_string(), document.clone());
    map.insert(
        "style".to_string(),
        Value::String(template.render.css.clone()),
    );
    Value::Object(map)
}

fn backup_existing_html(workspace: &Workspace) -> Result<Option<PathBuf>> {
    if !workspace.html_path.is_file() {
        return Ok(None);
    }
    fs::create_dir_all(&workspace.backups_dir)?;
    let backup = workspace
        .backups_dir
        .join(format!("document-{}.html", timestamp_suffix()));
    fs::copy(&workspace.html_path, &backup).with_context(|| {
        format!(
            "failed to back up {} to {}",
            workspace.html_path.display(),
            backup.display()
        )
    })?;
    Ok(Some(backup))
}

fn timestamp_suffix() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    seconds.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document_model::set_path, model::classic_template};
    use serde_json::json;

    #[test]
    fn renders_classic_template_with_escaped_content() {
        let template = classic_template();
        let mut document = template.defaults.clone();
        set_path(&mut document, "profile.full_name", json!("<Person>")).unwrap();
        set_path(&mut document, "profile.title", json!("Engineer")).unwrap();
        let html = render_html_string(&template, &document).unwrap();
        assert!(html.contains("&lt;Person&gt;"));
        assert!(html.contains("Engineer"));
        assert!(html.contains("@media screen and (max-width: 720px)"));
    }
}
