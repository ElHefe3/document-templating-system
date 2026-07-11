use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::{model::TEMPLATE_MANIFEST_FILE, templates::files::file_name_eq};

pub(crate) fn find_template_manifest_path_in_dir(path: &Path) -> Result<Option<PathBuf>> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("failed to read {}", path.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();

    for name in [TEMPLATE_MANIFEST_FILE, "template.json", "manifest.json"] {
        if let Some(candidate) = entries
            .iter()
            .find(|path| path.is_file() && file_name_eq(path, name))
        {
            return Ok(Some(candidate.clone()));
        }
    }

    let dirname_json = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|name| format!("{name}.json"));
    if let Some(dirname_json) = dirname_json {
        if let Some(candidate) = entries
            .iter()
            .find(|path| path.is_file() && file_name_eq(path, &dirname_json))
        {
            return Ok(Some(candidate.clone()));
        }
    }

    let json_files = entries
        .into_iter()
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .collect::<Vec<_>>();
    match json_files.as_slice() {
        [only] => Ok(Some(only.clone())),
        [] => Ok(None),
        _ => bail!(
            "template folder has multiple JSON files under {}; rename the manifest to template.json (found: {})",
            path.display(),
            display_path_names(&json_files)
        ),
    }
}

pub(crate) fn render_file_path(
    root: &Path,
    extension: &str,
    template_id: &str,
    fallback_names: &[&str],
) -> Result<Option<PathBuf>> {
    let id_file = root.join(format!("{template_id}.{extension}"));
    if id_file.is_file() {
        return Ok(Some(id_file));
    }
    for name in fallback_names {
        let candidate = root.join(name);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }
    let files = fs::read_dir(root)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| {
                    value.eq_ignore_ascii_case(extension)
                        || (extension == "html" && value.eq_ignore_ascii_case("htm"))
                })
        })
        .collect::<Vec<_>>();
    Ok(match files.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    })
}

fn display_path_names(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .filter_map(|path| path.file_name().and_then(|value| value.to_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    #[test]
    fn manifest_discovery_rejects_ambiguous_json_files() {
        let root = temp_dir("ambiguous-json");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.json"), "{}").unwrap();
        fs::write(root.join("b.json"), "{}").unwrap();

        let err = find_template_manifest_path_in_dir(&root).unwrap_err();

        assert!(err.to_string().contains("multiple JSON files"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_discovery_prefers_canonical_manifest_name() {
        let root = temp_dir("canonical-manifest");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(TEMPLATE_MANIFEST_FILE), "{}").unwrap();
        fs::write(root.join("metadata.json"), "{}").unwrap();

        let manifest = find_template_manifest_path_in_dir(&root).unwrap().unwrap();

        assert_eq!(manifest.file_name().unwrap(), TEMPLATE_MANIFEST_FILE);

        fs::remove_dir_all(root).unwrap();
    }
}
