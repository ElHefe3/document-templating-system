use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::archive::{self, ZipFile};

pub const TEMPLATE_PACKAGE_EXTENSION: &str = "document-template";

pub fn export_template_package(source_dir: &Path, output_path: &Path) -> Result<()> {
    let output = archive::encode_stored(collect_files(source_dir, source_dir)?)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, output)
        .with_context(|| format!("failed to write {}", output_path.display()))
}

pub fn extract_template_package(package_path: &Path, dest_dir: &Path) -> Result<()> {
    let bytes = fs::read(package_path)
        .with_context(|| format!("failed to read {}", package_path.display()))?;
    let files = archive::decode(&bytes)?;

    fs::create_dir_all(dest_dir)?;
    for file in files {
        let destination = safe_package_path(dest_dir, &file.name)?;
        if file.name.ends_with('/') {
            fs::create_dir_all(&destination)?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, file.body)
            .with_context(|| format!("failed to write {}", destination.display()))?;
    }
    Ok(())
}

fn collect_files(root: &Path, dir: &Path) -> Result<Vec<ZipFile>> {
    let mut files = Vec::new();
    collect_files_into(root, dir, &mut files)?;
    files.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(files)
}

fn collect_files_into(root: &Path, dir: &Path, files: &mut Vec<ZipFile>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files_into(root, &path, files)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let name = archive::zip_name_from_path(root, &path)?;
        let body = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        files.push(ZipFile { name, body });
    }
    Ok(())
}

fn safe_package_path(root: &Path, name: &str) -> Result<PathBuf> {
    let relative = Path::new(name);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        bail!("template package path must stay inside extraction folder: {name}");
    }
    Ok(root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn exports_and_extracts_template_package() {
        let root = temp_dir("package");
        let source = root.join("source");
        let extracted = root.join("extracted");
        fs::create_dir_all(source.join("assets")).unwrap();
        fs::write(source.join("document-template.json"), "{}").unwrap();
        fs::write(source.join("template.html"), "<main></main>").unwrap();
        fs::write(source.join("assets").join("preview.txt"), "preview").unwrap();

        let package = root.join("template.document-template");
        export_template_package(&source, &package).unwrap();
        extract_template_package(&package, &extracted).unwrap();

        assert_eq!(
            fs::read_to_string(extracted.join("template.html")).unwrap(),
            "<main></main>"
        );
        assert_eq!(
            fs::read_to_string(extracted.join("assets").join("preview.txt")).unwrap(),
            "preview"
        );

        fs::remove_dir_all(root).unwrap();
    }
}
