mod discovery;
mod manifest;
mod manifest_discovery;
mod renderer;
mod renderer_discovery;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;

use crate::{
    model::{load_template_from_file, TemplateManifest},
    templates::bundle::discovery::template_bundle_root,
    templates::bundle::manifest::{canonical_manifest_body, load_manifest_template_bundle},
    templates::bundle::manifest_discovery::find_template_manifest_path_in_dir,
    templates::bundle::renderer::load_renderer_only_template_bundle,
    templates::files::{collect_template_bundle_files, TemplateBundleFile},
    templates::package::{extract_template_package, TEMPLATE_PACKAGE_EXTENSION},
};

#[derive(Debug, Clone)]
pub struct TemplateBundle {
    pub template: TemplateManifest,
    pub layout: TemplateBundleLayout,
    pub manifest_body: Vec<u8>,
    pub files: Vec<TemplateBundleFile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateBundleLayout {
    SingleFile,
    Folder,
}

pub fn load_template_bundle(path: &Path) -> Result<TemplateBundle> {
    if path.is_dir() {
        load_template_folder_bundle(path)
    } else if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(TEMPLATE_PACKAGE_EXTENSION))
    {
        let temp_dir = temp_extract_dir("template-package");
        extract_template_package(path, &temp_dir)?;
        let result = load_template_folder_bundle(&temp_dir);
        let _ = fs::remove_dir_all(&temp_dir);
        result
    } else {
        let template = load_template_from_file(path)?;
        Ok(TemplateBundle {
            manifest_body: serde_json::to_vec_pretty(&template)?,
            template,
            layout: TemplateBundleLayout::SingleFile,
            files: Vec::new(),
        })
    }
}

fn temp_extract_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "document-templating-system-{prefix}-{}",
        timestamp_suffix()
    ))
}

fn load_template_folder_bundle(path: &Path) -> Result<TemplateBundle> {
    let bundle_root = template_bundle_root(path)?;
    let manifest_path = find_template_manifest_path_in_dir(&bundle_root)?;
    let template = match &manifest_path {
        Some(manifest_path) => load_manifest_template_bundle(&bundle_root, manifest_path)?,
        None => load_renderer_only_template_bundle(&bundle_root)?,
    };
    let manifest_body = canonical_manifest_body(&template)?;

    let files = collect_template_bundle_files(&bundle_root, manifest_path.as_deref())?;

    Ok(TemplateBundle {
        template,
        layout: TemplateBundleLayout::Folder,
        manifest_body,
        files,
    })
}

fn timestamp_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn loads_template_folder_bundle_with_sibling_renderer_files() {
        let root = temp_dir("folder-template");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("template.json"),
            r#"{"schema_version":1,"name":"Folder Template","defaults":{},"render":{"pdf_filename":""}}"#,
        )
        .unwrap();
        fs::write(
            root.join("template.html"),
            "<main>{{profile.full_name}}</main>",
        )
        .unwrap();
        fs::write(root.join("style.css"), "body { color: black; }").unwrap();

        let bundle = load_template_bundle(&root).unwrap();

        assert!(matches!(bundle.layout, TemplateBundleLayout::Folder));
        assert_eq!(
            bundle.template.id,
            root.file_name().and_then(|value| value.to_str()).unwrap()
        );
        assert_eq!(
            bundle.template.render.html,
            "<main>{{profile.full_name}}</main>"
        );
        assert_eq!(bundle.template.render.css, "body { color: black; }");
        assert_eq!(bundle.template.render.pdf_filename, "document.pdf");
        let mut file_names = bundle
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>();
        file_names.sort();
        assert_eq!(file_names, vec!["style.css", "template.html"]);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn uses_named_json_stem_as_folder_bundle_id() {
        let root = temp_dir("named-template");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("acme-cv.json"),
            r#"{"schema_version":1,"name":"Acme CV","defaults":{},"render":{}}"#,
        )
        .unwrap();
        fs::write(root.join("acme-cv.html"), "<main>Acme</main>").unwrap();

        let bundle = load_template_bundle(&root).unwrap();

        assert_eq!(bundle.template.id, "acme-cv");
        assert_eq!(bundle.template.render.html, "<main>Acme</main>");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loads_renderer_only_folder_bundle_without_json() {
        let parent = temp_dir("renderer-only");
        let root = parent.join("test_aa");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("resume-netherlands.html"), "<main>NL</main>").unwrap();
        fs::write(
            root.join("resume-netherlands.css"),
            ".nl-cv { color: #17202a; }",
        )
        .unwrap();

        let bundle = load_template_bundle(&root).unwrap();

        assert!(matches!(bundle.layout, TemplateBundleLayout::Folder));
        assert_eq!(
            bundle.template.id,
            root.file_name().and_then(|value| value.to_str()).unwrap()
        );
        assert_eq!(bundle.template.name, "Test Aa");
        assert_eq!(bundle.template.render.html, "<main>NL</main>");
        assert_eq!(bundle.template.render.css, ".nl-cv { color: #17202a; }");
        assert_eq!(bundle.template.render.pdf_filename, "test_aa.pdf");
        assert!(!bundle.template.sections.is_empty());
        let mut file_names = bundle
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>();
        file_names.sort();
        assert_eq!(
            file_names,
            vec!["resume-netherlands.css", "resume-netherlands.html"]
        );

        fs::remove_dir_all(parent).unwrap();
    }
}
