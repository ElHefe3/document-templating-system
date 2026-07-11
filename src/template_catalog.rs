use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use crate::{
    builtin_templates::{classic_template, country_resume_template_by_id},
    json_file::read_json,
    model::{TemplateManifest, TemplateSummary, BUILTIN_CLASSIC_ID, TEMPLATE_MANIFEST_FILE},
    template_manifest_validation::{hydrate_template_render_files, validate_template_manifest},
};

pub fn available_templates(workspace_root: Option<&Path>) -> Vec<TemplateSummary> {
    let mut templates = vec![classic_template().summary_with_source(true, "built-in")];
    templates.extend(
        crate::builtin_templates::country_resume_templates()
            .map(|template| template.summary_with_source(true, "built-in")),
    );
    if let Some(root) = workspace_root {
        let templates_dir = root.join("templates");
        if let Ok(entries) = fs::read_dir(templates_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(template) = load_template_from_folder(&path) {
                        templates.push(template.summary_with_source(false, "workspace"));
                    }
                } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
                    if let Ok(template) = load_template_from_file(&path) {
                        templates.push(template.summary_with_source(false, "workspace"));
                    }
                }
            }
        }
    }
    templates
}

pub fn load_template(workspace_root: &Path, template_ref: &str) -> Result<TemplateManifest> {
    if template_ref == BUILTIN_CLASSIC_ID {
        return Ok(classic_template());
    }
    if let Some(template) = country_resume_template_by_id(template_ref) {
        return Ok(template);
    }

    let direct = PathBuf::from(template_ref);
    if direct.is_file() {
        return load_template_from_file(&direct);
    }
    if direct.is_dir() {
        return load_template_from_folder(&direct);
    }

    let workspace_template = workspace_root
        .join("templates")
        .join(format!("{template_ref}.json"));
    if workspace_template.is_file() {
        return load_template_from_file(&workspace_template);
    }
    let workspace_template_folder = workspace_root.join("templates").join(template_ref);
    if workspace_template_folder
        .join(TEMPLATE_MANIFEST_FILE)
        .is_file()
    {
        return load_template_from_folder(&workspace_template_folder);
    }

    bail!("unknown template: {template_ref}")
}

pub fn local_template_exists(workspace_root: &Path, template_id: &str) -> bool {
    template_id == BUILTIN_CLASSIC_ID
        || country_resume_template_by_id(template_id).is_some()
        || workspace_root
            .join("templates")
            .join(format!("{template_id}.json"))
            .is_file()
        || workspace_root
            .join("templates")
            .join(template_id)
            .join(TEMPLATE_MANIFEST_FILE)
            .is_file()
}

pub fn load_template_from_file(path: &Path) -> Result<TemplateManifest> {
    let mut template: TemplateManifest = read_json(path)?;
    hydrate_template_render_files(&mut template, path.parent())?;
    validate_template_manifest(&mut template, path)?;
    Ok(template)
}

pub fn load_template_from_folder(path: &Path) -> Result<TemplateManifest> {
    let manifest_path = path.join(TEMPLATE_MANIFEST_FILE);
    let mut template: TemplateManifest = read_json(&manifest_path)?;
    hydrate_template_render_files(&mut template, Some(path))?;
    validate_template_manifest(&mut template, &manifest_path)?;
    Ok(template)
}
