use anyhow::{bail, Result};

use crate::model::TEMPLATE_MANIFEST_FILE;

#[derive(Debug, Clone)]
pub struct RemoteTemplateKeys {
    prefix: String,
}

impl RemoteTemplateKeys {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: normalize_prefix(prefix.into()),
        }
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn file_key(&self, template_id: &str) -> String {
        format!("{}{template_id}.json", self.prefix)
    }

    pub fn folder_prefix(&self, template_id: &str) -> String {
        format!("{}{template_id}/", self.prefix)
    }

    pub fn folder_manifest_key(&self, template_id: &str) -> String {
        format!("{}{template_id}/{TEMPLATE_MANIFEST_FILE}", self.prefix)
    }

    pub fn candidate_keys(&self, template_id: &str) -> Vec<String> {
        vec![
            self.folder_manifest_key(template_id),
            self.file_key(template_id),
            format!("{}{template_id}/template.json", self.prefix),
            format!("{}{template_id}/{template_id}.json", self.prefix),
        ]
    }

    pub fn remote_relative_key(&self, manifest_key: &str, relative_path: &str) -> Result<String> {
        let relative_path = relative_path.replace('\\', "/");
        if relative_path.starts_with('/') || relative_path.split('/').any(|part| part == "..") {
            bail!(
                "remote template file path must stay inside the template folder: {relative_path}"
            );
        }
        let base = manifest_key
            .rsplit_once('/')
            .map(|(base, _)| format!("{base}/"))
            .unwrap_or_default();
        Ok(format!("{base}{relative_path}"))
    }
}

fn normalize_prefix(mut prefix: String) -> String {
    prefix = prefix.trim_start_matches('/').to_string();
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/');
    }
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_prefix_shape() {
        assert_eq!(RemoteTemplateKeys::new("/templates").prefix(), "templates/");
        assert_eq!(RemoteTemplateKeys::new("templates/").prefix(), "templates/");
        assert_eq!(RemoteTemplateKeys::new("").prefix(), "");
    }

    #[test]
    fn builds_template_candidate_keys() {
        let keys = RemoteTemplateKeys::new("templates");

        assert_eq!(
            keys.candidate_keys("custom"),
            vec![
                "templates/custom/document-template.json",
                "templates/custom.json",
                "templates/custom/template.json",
                "templates/custom/custom.json",
            ]
        );
    }

    #[test]
    fn remote_relative_key_stays_under_manifest_folder() {
        let keys = RemoteTemplateKeys::new("templates");

        assert_eq!(
            keys.remote_relative_key("templates/custom/document-template.json", "style.css")
                .unwrap(),
            "templates/custom/style.css"
        );
        assert!(keys
            .remote_relative_key("templates/custom/document-template.json", "../style.css")
            .unwrap_err()
            .to_string()
            .contains("must stay inside"));
    }
}
