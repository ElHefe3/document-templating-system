use anyhow::Result;

use crate::{
    app_paths::Paths,
    model::{
        available_templates, load_template as load_local_template,
        set_workspace_template_with_manifest, TemplateManifest, TemplateSummary, Workspace,
    },
};

pub fn use_template(paths: &Paths, template_ref: &str) -> Result<Workspace> {
    let template = load_template(paths, template_ref)?;
    set_workspace_template_with_manifest(&paths.workspace, template_ref, &template)
}

pub fn list_templates(paths: Option<&Paths>) -> Vec<String> {
    match template_summaries(paths) {
        Ok(templates) => templates
            .into_iter()
            .map(|template| format!("{} - {} ({})", template.id, template.name, template.source))
            .collect(),
        Err(err) => vec![format!("Template list failed: {err}")],
    }
}

pub fn template_summaries(paths: Option<&Paths>) -> Result<Vec<TemplateSummary>> {
    let root = paths.map(|paths| paths.workspace.root.as_path());
    let mut templates = available_templates(root);
    if let Some(paths) = paths {
        templates.extend(crate::remote::actions::list_summaries(paths)?);
    }
    Ok(templates)
}

pub fn load_active_template(paths: &Paths) -> Result<TemplateManifest> {
    load_template(paths, &paths.workspace.active_template)
}

pub fn load_template(paths: &Paths, template_ref: &str) -> Result<TemplateManifest> {
    match load_local_template(&paths.workspace.root, template_ref) {
        Ok(template) => Ok(template),
        Err(local_error) => {
            if let Some(template) = crate::remote::actions::load_template(paths, template_ref)? {
                return Ok(template);
            }
            Err(local_error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_builtin_template_without_workspace() {
        let templates = list_templates(None);
        assert!(templates.iter().any(|line| line.contains("classic-resume")));
    }
}
