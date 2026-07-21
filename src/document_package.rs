use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use crate::{
    archive::{self, ZipFile},
    document_model::validate_document,
    model::{
        BUILTIN_CLASSIC_ID, BUILTIN_RESUME_GERMANY_ID, BUILTIN_RESUME_INDONESIA_ID,
        BUILTIN_RESUME_NETHERLANDS_ID, DOCUMENT_FILE, WORKSPACE_MANIFEST,
    },
    templates::catalog::load_template,
    workspace::{Workspace, WorkspaceManifest},
};

pub const DOCUMENT_PACKAGE_EXTENSION: &str = "dtsdoc";
pub const DOCUMENT_PACKAGE_MANIFEST: &str = "dtsdoc.json";

const FORMAT_NAME: &str = "document-templating-system.document";
const SCHEMA_VERSION: u32 = 1;
const INCLUDED_DIRS: &[&str] = &["assets", "templates"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPackageManifest {
    pub schema_version: u32,
    pub format: String,
    pub app_version: String,
    pub active_template: String,
    pub exported_at: String,
}

pub fn export_workspace_package(workspace: &Workspace, output_path: &Path) -> Result<()> {
    let bytes = export_workspace_package_bytes(workspace)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, bytes)
        .with_context(|| format!("failed to write {}", output_path.display()))
}

pub fn export_workspace_package_bytes(workspace: &Workspace) -> Result<Vec<u8>> {
    let mut files = Vec::new();
    files.push(ZipFile {
        name: DOCUMENT_PACKAGE_MANIFEST.to_string(),
        body: serde_json::to_vec_pretty(&DocumentPackageManifest {
            schema_version: SCHEMA_VERSION,
            format: FORMAT_NAME.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            active_template: workspace.active_template.clone(),
            exported_at: export_timestamp(),
        })?,
    });
    push_required_file(workspace, &mut files, WORKSPACE_MANIFEST)?;
    push_required_file(workspace, &mut files, DOCUMENT_FILE)?;
    for dir in INCLUDED_DIRS {
        collect_optional_dir(&workspace.root, &workspace.root.join(dir), &mut files)?;
    }
    collect_optional_file(
        &workspace.root,
        &workspace.renders_dir.join("document.html"),
        &mut files,
    )?;
    files.sort_by(|left, right| left.name.cmp(&right.name));
    archive::encode_stored(files)
}

pub fn import_workspace_package(package_path: &Path, dest_dir: &Path) -> Result<Workspace> {
    let bytes = fs::read(package_path)
        .with_context(|| format!("failed to read {}", package_path.display()))?;
    import_workspace_package_bytes(&bytes, dest_dir)
}

pub fn import_workspace_package_bytes(bytes: &[u8], dest_dir: &Path) -> Result<Workspace> {
    let files = archive::decode(bytes)?;
    validate_package_files(&files)?;
    if dest_dir.exists() && dest_dir.read_dir()?.next().is_some() {
        bail!(
            "document package import requires a new or empty workspace folder: {}",
            dest_dir.display()
        );
    }
    stage_and_validate_package(&files, "import")?;
    write_package_files(&files, dest_dir)?;
    Workspace::discover(dest_dir)
}

pub fn replace_workspace_from_package_bytes(
    bytes: &[u8],
    workspace: &Workspace,
) -> Result<Workspace> {
    let files = archive::decode(bytes)?;
    validate_package_files(&files)?;
    stage_and_validate_package(&files, "replace")?;
    clear_known_workspace_files(workspace)?;
    write_package_files(&files, &workspace.root)?;
    Workspace::discover(&workspace.root)
}

fn push_required_file(workspace: &Workspace, files: &mut Vec<ZipFile>, name: &str) -> Result<()> {
    let path = workspace.root.join(name);
    let body = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    files.push(ZipFile {
        name: name.to_string(),
        body,
    });
    Ok(())
}

fn collect_optional_dir(root: &Path, dir: &Path, files: &mut Vec<ZipFile>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_optional_dir(root, &path, files)?;
            continue;
        }
        collect_optional_file(root, &path, files)?;
    }
    Ok(())
}

fn collect_optional_file(root: &Path, path: &Path, files: &mut Vec<ZipFile>) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    files.push(ZipFile {
        name: archive::zip_name_from_path(root, path)?,
        body: fs::read(path).with_context(|| format!("failed to read {}", path.display()))?,
    });
    Ok(())
}

fn validate_package_files(files: &[ZipFile]) -> Result<()> {
    let manifest = required_json::<DocumentPackageManifest>(files, DOCUMENT_PACKAGE_MANIFEST)?;
    if manifest.schema_version != SCHEMA_VERSION {
        bail!(
            "unsupported document package schema version: {}",
            manifest.schema_version
        );
    }
    if manifest.format != FORMAT_NAME {
        bail!("unsupported document package format: {}", manifest.format);
    }
    let workspace_manifest = required_json::<WorkspaceManifest>(files, WORKSPACE_MANIFEST)?;
    if workspace_manifest.schema_version != SCHEMA_VERSION {
        bail!(
            "unsupported workspace schema version: {}",
            workspace_manifest.schema_version
        );
    }
    if workspace_manifest.active_template.trim().is_empty() {
        bail!("document package workspace active_template is required");
    }
    if !package_contains_template(files, &workspace_manifest.active_template) {
        bail!(
            "document package is missing active template: {}",
            workspace_manifest.active_template
        );
    }
    let _: Value = required_json(files, DOCUMENT_FILE)?;
    for file in files {
        safe_package_path(Path::new("."), &file.name)?;
    }
    Ok(())
}

fn package_contains_template(files: &[ZipFile], template_id: &str) -> bool {
    is_builtin_template(template_id)
        || files
            .iter()
            .any(|file| file.name == format!("templates/{template_id}.json"))
        || files
            .iter()
            .any(|file| file.name == format!("templates/{template_id}/document-template.json"))
}

fn is_builtin_template(template_id: &str) -> bool {
    matches!(
        template_id,
        BUILTIN_CLASSIC_ID
            | BUILTIN_RESUME_GERMANY_ID
            | BUILTIN_RESUME_NETHERLANDS_ID
            | BUILTIN_RESUME_INDONESIA_ID
    )
}

fn validate_imported_workspace(root: &Path) -> Result<()> {
    let workspace = Workspace::discover(root)?;
    let template = load_template(&workspace.root, &workspace.active_template)?;
    let document = workspace.load_document()?;
    validate_document(&template, &document)
}

fn stage_and_validate_package(files: &[ZipFile], label: &str) -> Result<()> {
    let staging_dir = package_staging_dir(label);
    let result = (|| -> Result<()> {
        write_package_files(files, &staging_dir)?;
        validate_imported_workspace(&staging_dir)
    })();
    let cleanup = fs::remove_dir_all(&staging_dir);
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(err)) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        (Ok(()), Err(err)) => Err(err).context("failed to remove document package staging folder"),
        (Err(err), _) => Err(err),
    }
}

fn required_json<T>(files: &[ZipFile], name: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let file = files
        .iter()
        .find(|file| file.name == name)
        .with_context(|| format!("document package is missing {name}"))?;
    serde_json::from_slice(&file.body).with_context(|| format!("failed to parse {name}"))
}

fn write_package_files(files: &[ZipFile], dest_dir: &Path) -> Result<()> {
    fs::create_dir_all(dest_dir)
        .with_context(|| format!("failed to create {}", dest_dir.display()))?;
    for file in files {
        if file.name == DOCUMENT_PACKAGE_MANIFEST {
            continue;
        }
        let destination = safe_package_path(dest_dir, &file.name)?;
        if file.name.ends_with('/') {
            fs::create_dir_all(&destination)?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, &file.body)
            .with_context(|| format!("failed to write {}", destination.display()))?;
    }
    Ok(())
}

fn clear_known_workspace_files(workspace: &Workspace) -> Result<()> {
    remove_file_if_exists(&workspace.root.join(WORKSPACE_MANIFEST))?;
    remove_file_if_exists(&workspace.root.join(DOCUMENT_FILE))?;
    remove_dir_if_exists(&workspace.assets_dir)?;
    remove_dir_if_exists(&workspace.renders_dir)?;
    remove_dir_if_exists(&workspace.root.join("templates"))?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    if path.is_file() {
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn safe_package_path(root: &Path, name: &str) -> Result<PathBuf> {
    let relative = Path::new(name);
    if name.contains('\\')
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        bail!("document package path must stay inside workspace: {name}");
    }
    Ok(root.join(relative))
}

fn export_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn package_staging_dir(label: &str) -> PathBuf {
    let suffix = OffsetDateTime::now_utc().unix_timestamp_nanos().to_string();
    std::env::temp_dir().join(format!("document-templating-system-{label}-{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        archive,
        test_support::temp_dir,
        workspace::{init_workspace, set_workspace_template_with_manifest},
    };
    use std::fs;

    #[test]
    fn package_round_trips_workspace_files() {
        let root = temp_dir("dtsdoc-roundtrip");
        let source = root.join("source");
        let dest = root.join("dest");
        let workspace = init_workspace(&source, None).unwrap();
        fs::write(workspace.assets_dir.join("avatar.svg"), "<svg></svg>").unwrap();
        fs::write(workspace.html_path.clone(), "<main>Rendered</main>").unwrap();

        let bytes = export_workspace_package_bytes(&workspace).unwrap();
        let imported = import_workspace_package_bytes(&bytes, &dest).unwrap();

        assert_eq!(imported.active_template, workspace.active_template);
        assert!(dest.join(DOCUMENT_FILE).is_file());
        assert_eq!(
            fs::read_to_string(dest.join("assets").join("avatar.svg")).unwrap(),
            "<svg></svg>"
        );
        assert_eq!(
            fs::read_to_string(dest.join("renders").join("document.html")).unwrap(),
            "<main>Rendered</main>"
        );

        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn import_rejects_non_empty_destination() {
        let root = temp_dir("dtsdoc-non-empty");
        let source = root.join("source");
        let dest = root.join("dest");
        let workspace = init_workspace(&source, None).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("note.txt"), "existing").unwrap();

        let bytes = export_workspace_package_bytes(&workspace).unwrap();
        let error = import_workspace_package_bytes(&bytes, &dest).unwrap_err();

        assert!(error.to_string().contains("new or empty workspace"));
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn import_rejects_unsafe_paths() {
        let files = vec![
            ZipFile {
                name: DOCUMENT_PACKAGE_MANIFEST.to_string(),
                body: serde_json::to_vec(&DocumentPackageManifest {
                    schema_version: 1,
                    format: FORMAT_NAME.to_string(),
                    app_version: "0.1.0".to_string(),
                    active_template: "classic-resume".to_string(),
                    exported_at: "1970-01-01T00:00:00Z".to_string(),
                })
                .unwrap(),
            },
            ZipFile {
                name: WORKSPACE_MANIFEST.to_string(),
                body: br#"{"schema_version":1,"active_template":"classic-resume"}"#.to_vec(),
            },
            ZipFile {
                name: DOCUMENT_FILE.to_string(),
                body: b"{}".to_vec(),
            },
            ZipFile {
                name: "../bad.txt".to_string(),
                body: b"bad".to_vec(),
            },
        ];
        let bytes = archive::encode_stored(files).unwrap();
        let root = temp_dir("dtsdoc-bad-path");

        let error = import_workspace_package_bytes(&bytes, &root).unwrap_err();

        assert!(error.to_string().contains("must stay inside workspace"));
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn import_rejects_bad_manifest() {
        let bytes = archive::encode_stored(vec![ZipFile {
            name: DOCUMENT_PACKAGE_MANIFEST.to_string(),
            body: br#"{"schema_version":99,"format":"document-templating-system.document","app_version":"0.1.0","active_template":"classic-resume","exported_at":"now"}"#.to_vec(),
        }])
        .unwrap();
        let root = temp_dir("dtsdoc-bad-manifest");

        let error = import_workspace_package_bytes(&bytes, &root).unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported document package schema version"));
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn import_rejects_missing_active_template() {
        let files = vec![
            ZipFile {
                name: DOCUMENT_PACKAGE_MANIFEST.to_string(),
                body: serde_json::to_vec(&DocumentPackageManifest {
                    schema_version: 1,
                    format: FORMAT_NAME.to_string(),
                    app_version: "0.1.0".to_string(),
                    active_template: "custom".to_string(),
                    exported_at: "1970-01-01T00:00:00Z".to_string(),
                })
                .unwrap(),
            },
            ZipFile {
                name: WORKSPACE_MANIFEST.to_string(),
                body: br#"{"schema_version":1,"active_template":"custom"}"#.to_vec(),
            },
            ZipFile {
                name: DOCUMENT_FILE.to_string(),
                body: b"{}".to_vec(),
            },
        ];
        let bytes = archive::encode_stored(files).unwrap();
        let root = temp_dir("dtsdoc-missing-template");

        let error = import_workspace_package_bytes(&bytes, &root).unwrap_err();

        assert!(error
            .to_string()
            .contains("document package is missing active template"));
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn custom_workspace_templates_are_included() {
        let root = temp_dir("dtsdoc-template");
        let source = root.join("source");
        let dest = root.join("dest");
        let workspace = init_workspace(&source, None).unwrap();
        let template_dir = source.join("templates").join("custom");
        fs::create_dir_all(&template_dir).unwrap();
        fs::write(
            template_dir.join("document-template.json"),
            r#"{"schema_version":1,"id":"custom","name":"Custom","render":{"html":"<main>{{name}}</main>","css":"","pdf_filename":"custom.pdf"},"sections":[],"defaults":{}}"#,
        )
        .unwrap();
        let template = crate::templates::catalog::load_template(&source, "custom").unwrap();
        let workspace =
            set_workspace_template_with_manifest(&workspace, "custom", &template).unwrap();

        let bytes = export_workspace_package_bytes(&workspace).unwrap();
        import_workspace_package_bytes(&bytes, &dest).unwrap();

        assert!(dest
            .join("templates")
            .join("custom")
            .join("document-template.json")
            .is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
