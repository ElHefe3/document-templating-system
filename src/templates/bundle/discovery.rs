use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::{
    templates::bundle::manifest_discovery::find_template_manifest_path_in_dir,
    templates::bundle::renderer_discovery::find_renderer_only_files,
};

pub(crate) fn template_bundle_root(path: &Path) -> Result<PathBuf> {
    if is_template_bundle_dir(path)? {
        return Ok(path.to_path_buf());
    }

    let mut template_dirs = Vec::new();
    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let child = entry?.path();
        if child.is_dir() && is_template_bundle_dir(&child)? {
            template_dirs.push(child);
        }
    }
    match template_dirs.as_slice() {
        [only] => Ok(only.clone()),
        [] => bail!(
            "template folder must contain a template JSON file or one HTML renderer file: {} (looked for document-template.json, template.json, manifest.json, <folder-name>.json, a single .json file, or a single .html/.htm file; found: {})",
            path.display(),
            describe_dir_entries(path)?
        ),
        _ => bail!(
            "template folder has multiple template subfolders under {}; select one folder or add template.json at the selected root",
            path.display()
        ),
    }
}

fn is_template_bundle_dir(path: &Path) -> Result<bool> {
    Ok(find_template_manifest_path_in_dir(path)?.is_some()
        || find_renderer_only_files(path)?.is_some())
}

fn describe_dir_entries(path: &Path) -> Result<String> {
    let mut names = fs::read_dir(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let mut name = entry.file_name().to_str()?.to_string();
            if entry.path().is_dir() {
                name.push('/');
            }
            Some(name)
        })
        .collect::<Vec<_>>();
    names.sort();
    if names.is_empty() {
        Ok("empty folder".to_string())
    } else {
        let extra = names.len().saturating_sub(12);
        names.truncate(12);
        let mut description = names.join(", ");
        if extra > 0 {
            description.push_str(&format!(", ... and {extra} more"));
        }
        Ok(description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn template_bundle_root_selects_single_nested_template_folder() {
        let root = temp_dir("nested-template-root");
        let child = root.join("template");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&child).unwrap();
        fs::write(child.join("template.html"), "<main></main>").unwrap();

        let selected = template_bundle_root(&root).unwrap();

        assert_eq!(selected, child);

        fs::remove_dir_all(root).unwrap();
    }
}
