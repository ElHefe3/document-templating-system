use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    model::{TemplateManifest, TemplateRender, BUILTIN_CLASSIC_ID, TEMPLATE_MANIFEST_FILE},
    templates::{
        builtin::{classic_template, country_resume_template_by_id},
        bundle::renderer_discovery::{file_stem_string, find_renderer_only_files},
        manifest_validation::validate_template_manifest,
    },
};

pub(crate) fn load_renderer_only_template_bundle(root: &Path) -> Result<TemplateManifest> {
    let files = find_renderer_only_files(root)?.with_context(|| {
        format!(
            "template folder has no HTML renderer file: {}",
            root.display()
        )
    })?;
    let id = root
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .or_else(|| file_stem_string(&files.html_path))
        .context("template id is required")?;
    let html_stem = file_stem_string(&files.html_path);
    let mut template = html_stem
        .as_deref()
        .and_then(template_base_for_id)
        .unwrap_or_else(|| empty_renderer_template(&id));

    template.id = id.clone();
    template.name = display_name_from_id(&id);
    if template.description.trim().is_empty() {
        template.description = "Uploaded renderer-only template bundle.".to_string();
    }
    template.render.html = fs::read_to_string(&files.html_path)
        .with_context(|| format!("failed to read {}", files.html_path.display()))?;
    template.render.css = match &files.css_path {
        Some(path) => fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?,
        None => String::new(),
    };
    template.render.html_file = file_name_string(&files.html_path);
    template.render.css_file = files
        .css_path
        .as_ref()
        .and_then(|path| file_name_string(path));
    template.render.pdf_filename = format!("{id}.pdf");
    validate_template_manifest(&mut template, &root.join(TEMPLATE_MANIFEST_FILE))?;
    Ok(template)
}

fn empty_renderer_template(id: &str) -> TemplateManifest {
    TemplateManifest {
        schema_version: 1,
        id: id.to_string(),
        name: display_name_from_id(id),
        version: Some("0.1.0".to_string()),
        description: "Uploaded renderer-only template bundle.".to_string(),
        sections: Vec::new(),
        defaults: Value::Object(serde_json::Map::new()),
        render: TemplateRender {
            html: String::new(),
            css: String::new(),
            html_file: None,
            css_file: None,
            pdf_filename: String::new(),
        },
    }
}

fn template_base_for_id(template_id: &str) -> Option<TemplateManifest> {
    if template_id == BUILTIN_CLASSIC_ID {
        Some(classic_template())
    } else {
        country_resume_template_by_id(template_id)
    }
}

fn display_name_from_id(template_id: &str) -> String {
    template_id
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn file_name_string(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
}
