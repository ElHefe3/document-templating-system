use std::{fs, path::Path};

use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateBundleFile {
    pub name: String,
    pub body: Vec<u8>,
    pub content_type: String,
}

pub(crate) fn collect_template_bundle_files(
    root: &Path,
    manifest_path: Option<&Path>,
) -> Result<Vec<TemplateBundleFile>> {
    let mut files = Vec::new();
    collect_template_bundle_files_inner(root, root, manifest_path, &mut files)?;
    files.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(files)
}

fn collect_template_bundle_files_inner(
    root: &Path,
    dir: &Path,
    manifest_path: Option<&Path>,
    files: &mut Vec<TemplateBundleFile>,
) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_template_bundle_files_inner(root, &path, manifest_path, files)?;
            continue;
        }
        if !path.is_file() || manifest_path.is_some_and(|manifest| path == manifest) {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("failed to build relative path for {}", path.display()))?;
        let name = path_name(relative);
        files.push(TemplateBundleFile {
            name,
            body: fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?,
            content_type: content_type_for_template_file(&path).to_string(),
        });
    }
    Ok(())
}

fn path_name(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn file_name_eq(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn content_type_for_template_file(path: &Path) -> &'static str {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return "application/octet-stream";
    };
    if extension.eq_ignore_ascii_case("css") {
        "text/css; charset=utf-8"
    } else if is_html_extension(extension) {
        "text/html; charset=utf-8"
    } else if extension.eq_ignore_ascii_case("json") {
        "application/json; charset=utf-8"
    } else if extension.eq_ignore_ascii_case("png") {
        "image/png"
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        "image/jpeg"
    } else if extension.eq_ignore_ascii_case("webp") {
        "image/webp"
    } else if extension.eq_ignore_ascii_case("svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

pub(crate) fn is_html_extension(extension: &str) -> bool {
    extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
}
