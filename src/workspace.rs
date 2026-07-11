use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    json_file::{atomic_write_json, read_json},
    model::{TemplateManifest, BUILTIN_CLASSIC_ID, DOCUMENT_FILE, WORKSPACE_MANIFEST},
    template_catalog::load_template,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub schema_version: u32,
    pub active_template: String,
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub document_path: PathBuf,
    pub assets_dir: PathBuf,
    pub renders_dir: PathBuf,
    pub html_path: PathBuf,
    pub backups_dir: PathBuf,
    pub outputs_dir: PathBuf,
    pub active_template: String,
}

impl Workspace {
    pub fn discover(root: &Path) -> Result<Self> {
        let manifest_path = root.join(WORKSPACE_MANIFEST);
        if manifest_path.is_file() {
            let manifest = read_workspace_manifest(&manifest_path)?;
            return Ok(Self {
                root: root.to_path_buf(),
                document_path: root.join(DOCUMENT_FILE),
                assets_dir: root.join("assets"),
                renders_dir: root.join("renders"),
                html_path: root.join("renders").join("document.html"),
                backups_dir: root.join("backups"),
                outputs_dir: root.join("outputs"),
                active_template: manifest.active_template,
            });
        }

        bail!(
            "workspace is missing {WORKSPACE_MANIFEST}: {}",
            root.display()
        )
    }

    pub fn load_document(&self) -> Result<Value> {
        if !self.document_path.is_file() {
            bail!("missing document: {}", self.document_path.display());
        }
        read_json(&self.document_path)
    }

    pub fn save_document(&self, document: &Value) -> Result<()> {
        atomic_write_json(&self.document_path, document)
    }

    pub fn asset_reference(&self, filename: &str) -> String {
        format!("../assets/{filename}")
    }

    pub fn pdf_output_path(&self, template: &TemplateManifest) -> PathBuf {
        self.outputs_dir.join(&template.render.pdf_filename)
    }
}

pub fn init_workspace(root: &Path, template_ref: Option<&str>) -> Result<Workspace> {
    let template_id = template_ref.unwrap_or(BUILTIN_CLASSIC_ID);
    let template = load_template(root, template_id)?;
    fs::create_dir_all(root).with_context(|| format!("failed to create {}", root.display()))?;
    fs::create_dir_all(root.join("assets"))?;
    fs::create_dir_all(root.join("renders"))?;
    fs::create_dir_all(root.join("outputs"))?;
    fs::create_dir_all(root.join("backups"))?;

    let manifest = WorkspaceManifest {
        schema_version: 1,
        active_template: template_id.to_string(),
    };
    atomic_write_json(
        &root.join(WORKSPACE_MANIFEST),
        &serde_json::to_value(manifest)?,
    )?;
    atomic_write_json(&root.join(DOCUMENT_FILE), &template.defaults)?;
    Workspace::discover(root)
}

pub fn set_workspace_template_with_manifest(
    workspace: &Workspace,
    template_ref: &str,
    template: &TemplateManifest,
) -> Result<Workspace> {
    if workspace.active_template == template_ref {
        return Ok(workspace.clone());
    }
    fs::create_dir_all(workspace.root.join("assets"))?;
    fs::create_dir_all(workspace.root.join("renders"))?;
    fs::create_dir_all(workspace.root.join("outputs"))?;
    fs::create_dir_all(workspace.root.join("backups"))?;

    let manifest_path = workspace.root.join(WORKSPACE_MANIFEST);
    let manifest = WorkspaceManifest {
        schema_version: 1,
        active_template: template_ref.to_string(),
    };
    atomic_write_json(
        &manifest_path,
        &serde_json::to_value(manifest).context("failed to serialize workspace manifest")?,
    )?;
    atomic_write_json(&workspace.root.join(DOCUMENT_FILE), &template.defaults)?;
    Workspace::discover(&workspace.root)
}

fn read_workspace_manifest(path: &Path) -> Result<WorkspaceManifest> {
    read_json(path)
}
