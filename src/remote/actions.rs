use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::{
    app_paths::Paths,
    config::StorageConfig,
    model::{local_template_exists, TemplateManifest, TemplateSummary},
    remote::templates::{RemoteTemplateStore, RemoteTemplateWrite},
    storage::{configured_prefix, provider_from_config, StorageProvider},
    templates::bundle::load_template_bundle,
};

#[derive(Debug, Clone)]
pub struct RemoteTemplatePreview {
    pub id: String,
    pub key: String,
    pub exists: bool,
}

pub fn storage_config(paths: &Paths) -> Option<&StorageConfig> {
    paths.app_config.storage.as_ref()
}

pub fn storage_summary(paths: &Paths) -> String {
    match storage_config(paths) {
        Some(config) => format!("{} configured", config.driver),
        None => "not configured".to_string(),
    }
}

pub fn list_summaries(paths: &Paths) -> Result<Vec<TemplateSummary>> {
    match remote_template_store(paths)? {
        Some(store) => store.list_summaries(),
        None => Ok(Vec::new()),
    }
}

pub fn load_template(paths: &Paths, template_ref: &str) -> Result<Option<TemplateManifest>> {
    let Some(store) = remote_template_store(paths)? else {
        return Ok(None);
    };
    store.load(template_ref)
}

pub fn remote_template_upload_preview(paths: &Paths, path: &Path) -> Result<RemoteTemplatePreview> {
    let bundle = load_template_bundle(path)?;
    ensure_remote_template_id_allowed(paths, &bundle.template.id)?;
    let store = required_remote_template_store(paths)?;
    let template_id = bundle.template.id.clone();
    let key = store.manifest_key(&bundle);
    Ok(RemoteTemplatePreview {
        exists: store.exists(&template_id)?,
        id: template_id,
        key,
    })
}

pub fn upload_remote_template(
    paths: &Paths,
    path: &Path,
    overwrite: bool,
) -> Result<RemoteTemplateWrite> {
    let bundle = load_template_bundle(path)?;
    ensure_remote_template_id_allowed(paths, &bundle.template.id)?;
    let store = required_remote_template_store(paths)?;
    if !overwrite && store.exists(&bundle.template.id)? {
        bail!("remote template already exists: {}", bundle.template.id);
    }
    store.upload(&bundle)
}

pub fn delete_remote_template(paths: &Paths, template_id: &str) -> Result<String> {
    required_remote_template_store(paths)?.delete(template_id)?;
    Ok(format!("Deleted remote template {template_id}."))
}

fn storage_provider(paths: &Paths) -> Result<Option<Box<dyn StorageProvider>>> {
    paths
        .app_config
        .storage
        .as_ref()
        .map(provider_from_config)
        .transpose()
}

fn ensure_remote_template_id_allowed(paths: &Paths, template_id: &str) -> Result<()> {
    if local_template_exists(&paths.workspace.root, template_id) {
        bail!("remote template id collides with a built-in or workspace template: {template_id}");
    }
    Ok(())
}

fn remote_template_store(paths: &Paths) -> Result<Option<RemoteTemplateStore>> {
    let Some(provider) = storage_provider(paths)? else {
        return Ok(None);
    };
    Ok(Some(RemoteTemplateStore::new(
        provider,
        remote_template_prefix(storage_config(paths)),
    )))
}

fn required_remote_template_store(paths: &Paths) -> Result<RemoteTemplateStore> {
    remote_template_store(paths)?.context("storage is not configured")
}

fn remote_template_prefix(config: Option<&StorageConfig>) -> String {
    configured_prefix(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::AppConfig, workspace::Workspace};
    use std::path::PathBuf;

    #[test]
    fn storage_summary_reports_missing_storage_config() {
        let paths = paths_without_storage();

        assert_eq!(storage_summary(&paths), "not configured");
    }

    #[test]
    fn list_summaries_without_storage_returns_empty() {
        let paths = paths_without_storage();

        assert!(list_summaries(&paths).unwrap().is_empty());
    }

    fn paths_without_storage() -> Paths {
        let root = PathBuf::from("C:/document-templating-system-test-workspace");
        Paths {
            project_root: PathBuf::from("C:/document-templating-system-test-project"),
            workspace: Workspace {
                root: root.clone(),
                document_path: root.join("document.json"),
                assets_dir: root.join("assets"),
                renders_dir: root.join("renders"),
                html_path: root.join("renders").join("document.html"),
                backups_dir: root.join("backups"),
                outputs_dir: root.join("outputs"),
                active_template: "classic-resume".to_string(),
            },
            app_config: AppConfig::default(),
        }
    }
}
