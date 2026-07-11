use std::{collections::BTreeSet, path::Path};

use anyhow::{bail, Context, Result};

use crate::{
    model::{
        validate_template_manifest, TemplateManifest, TemplateSummary, TEMPLATE_MANIFEST_FILE,
    },
    remote_template_keys::RemoteTemplateKeys,
    storage::StorageProvider,
    template_bundle::{TemplateBundle, TemplateBundleLayout},
};

#[derive(Debug, Clone)]
pub struct RemoteTemplateWrite {
    pub id: String,
    pub key: String,
    pub url: String,
}

pub struct RemoteTemplateStore {
    provider: Box<dyn StorageProvider>,
    keys: RemoteTemplateKeys,
}

impl RemoteTemplateStore {
    pub fn new(provider: Box<dyn StorageProvider>, prefix: impl Into<String>) -> Self {
        Self {
            provider,
            keys: RemoteTemplateKeys::new(prefix),
        }
    }

    pub fn list_summaries(&self) -> Result<Vec<TemplateSummary>> {
        let objects = self.provider.list(self.keys.prefix())?;
        let mut templates = Vec::new();
        let mut seen = BTreeSet::new();
        for canonical in [true, false] {
            for object in &objects {
                let is_canonical = object.key.ends_with(&format!("/{TEMPLATE_MANIFEST_FILE}"));
                if is_canonical != canonical || !object.key.ends_with(".json") {
                    continue;
                }
                let Ok(bytes) = self.provider.download(&object.key) else {
                    continue;
                };
                if let Ok(template) = self.template_from_bytes(&bytes, &object.key, false) {
                    if seen.insert(template.id.clone()) {
                        templates.push(template.summary_with_source(false, "remote"));
                    }
                }
            }
        }
        Ok(templates)
    }

    pub fn load(&self, template_id: &str) -> Result<Option<TemplateManifest>> {
        for key in self.keys.candidate_keys(template_id) {
            if let Ok(bytes) = self.provider.download(&key) {
                return Ok(Some(self.template_from_bytes(&bytes, &key, true)?));
            }
        }
        Ok(None)
    }

    pub fn exists(&self, template_id: &str) -> Result<bool> {
        let file_key = self.keys.file_key(template_id);
        let folder_prefix = self.keys.folder_prefix(template_id);
        let objects = self.provider.list(self.keys.prefix())?;
        Ok(objects
            .iter()
            .any(|object| object.key == file_key || object.key.starts_with(&folder_prefix)))
    }

    pub fn upload(&self, bundle: &TemplateBundle) -> Result<RemoteTemplateWrite> {
        let key = self.manifest_key(bundle);
        let template_id = bundle.template.id.clone();
        let url = self.provider.upload(
            &key,
            &bundle.manifest_body,
            "application/json; charset=utf-8",
        )?;
        for file in &bundle.files {
            self.provider.upload(
                &format!(
                    "{prefix}{template_id}/{}",
                    file.name,
                    prefix = self.keys.prefix()
                ),
                &file.body,
                &file.content_type,
            )?;
        }
        Ok(RemoteTemplateWrite {
            id: template_id,
            key,
            url,
        })
    }

    pub fn delete(&self, template_id: &str) -> Result<()> {
        for key in self.delete_keys(template_id)? {
            self.provider.delete(&key)?;
        }
        Ok(())
    }

    pub fn manifest_key(&self, bundle: &TemplateBundle) -> String {
        match bundle.layout {
            TemplateBundleLayout::SingleFile => self.keys.file_key(&bundle.template.id),
            TemplateBundleLayout::Folder => self.keys.folder_manifest_key(&bundle.template.id),
        }
    }

    fn template_from_bytes(
        &self,
        bytes: &[u8],
        key: &str,
        hydrate_files: bool,
    ) -> Result<TemplateManifest> {
        let mut template: TemplateManifest = serde_json::from_slice(bytes)
            .with_context(|| format!("failed to parse remote template {key}"))?;
        if hydrate_files {
            self.hydrate_files(&mut template, key)?;
            validate_template_manifest(&mut template, Path::new(key))
                .with_context(|| format!("remote template is invalid: {key}"))?;
        } else {
            if template.id.trim().is_empty() {
                bail!("remote template id is required: {key}");
            }
            if template.render.pdf_filename.trim().is_empty() {
                template.render.pdf_filename = "document.pdf".to_string();
            }
        }
        Ok(template)
    }

    fn hydrate_files(&self, template: &mut TemplateManifest, manifest_key: &str) -> Result<()> {
        if template.render.html.trim().is_empty() {
            if let Some(path) = &template.render.html_file {
                let key = self.keys.remote_relative_key(manifest_key, path)?;
                template.render.html = String::from_utf8(self.provider.download(&key)?)
                    .with_context(|| format!("remote template file is not UTF-8: {key}"))?;
            }
        }
        if template.render.css.trim().is_empty() {
            if let Some(path) = &template.render.css_file {
                let key = self.keys.remote_relative_key(manifest_key, path)?;
                template.render.css = String::from_utf8(self.provider.download(&key)?)
                    .with_context(|| format!("remote template file is not UTF-8: {key}"))?;
            }
        }
        Ok(())
    }

    fn delete_keys(&self, template_id: &str) -> Result<Vec<String>> {
        let folder_prefix = self.keys.folder_prefix(template_id);
        let mut keys = vec![self.keys.file_key(template_id)];
        keys.extend(
            self.provider
                .list(&folder_prefix)?
                .into_iter()
                .map(|object| object.key),
        );
        keys.sort();
        keys.dedup();
        Ok(keys)
    }
}
