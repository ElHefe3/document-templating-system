use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::model::TemplateManifest;

pub fn validate_template_manifest(template: &mut TemplateManifest, path: &Path) -> Result<()> {
    if template.id.trim().is_empty() {
        bail!("template id is required: {}", path.display());
    }
    if template.render.html.trim().is_empty() {
        bail!("template render.html is required: {}", path.display());
    }
    if template.render.pdf_filename.trim().is_empty() {
        template.render.pdf_filename = "document.pdf".to_string();
    }
    Ok(())
}

pub fn hydrate_template_render_files(
    template: &mut TemplateManifest,
    base_dir: Option<&Path>,
) -> Result<()> {
    let Some(base_dir) = base_dir else {
        return Ok(());
    };
    if template.render.html.trim().is_empty() {
        if let Some(path) = &template.render.html_file {
            let path = safe_template_file_path(base_dir, path)?;
            template.render.html = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
        }
    }
    if template.render.css.trim().is_empty() {
        if let Some(path) = &template.render.css_file {
            let path = safe_template_file_path(base_dir, path)?;
            template.render.css = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
        }
    }
    Ok(())
}

fn safe_template_file_path(base_dir: &Path, relative_path: &str) -> Result<PathBuf> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        bail!("template file path must stay inside the template folder: {relative_path}");
    }
    Ok(base_dir.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{TemplateRender, TEMPLATE_MANIFEST_FILE},
        template_catalog::load_template_from_folder,
        test_support::temp_dir,
    };
    use serde_json::json;

    #[test]
    fn loads_template_from_folder_with_render_file_refs() {
        let root = temp_dir("folder-render-refs");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join(TEMPLATE_MANIFEST_FILE),
            serde_json::to_string_pretty(&json!({
                "schema_version": 1,
                "id": "folder-render-refs",
                "name": "Folder Render Refs",
                "render": {
                    "html_file": "template.html",
                    "css_file": "style.css",
                    "pdf_filename": ""
                }
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(root.join("template.html"), "<main></main>").unwrap();
        fs::write(root.join("style.css"), "body {}").unwrap();

        let template = load_template_from_folder(&root).unwrap();

        assert_eq!(template.render.html, "<main></main>");
        assert_eq!(template.render.css, "body {}");
        assert_eq!(template.render.pdf_filename, "document.pdf");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_template_render_file_escape_paths() {
        let root = temp_dir("escape-render-ref");
        fs::create_dir_all(&root).unwrap();
        let mut template = TemplateManifest {
            schema_version: 1,
            id: "escape-render-ref".to_string(),
            name: "Escape Render Ref".to_string(),
            version: None,
            description: String::new(),
            sections: Vec::new(),
            defaults: serde_json::Value::Null,
            render: TemplateRender {
                html: String::new(),
                css: String::new(),
                html_file: Some("../template.html".to_string()),
                css_file: None,
                pdf_filename: "document.pdf".to_string(),
            },
        };

        let err = hydrate_template_render_files(&mut template, Some(&root)).unwrap_err();

        assert!(err
            .to_string()
            .contains("template file path must stay inside"));

        fs::remove_dir_all(root).unwrap();
    }
}
